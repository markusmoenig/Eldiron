//! View-owned material atlases. Procedural float maps are converted on the GPU;
//! authoritative ruleset/library CPU bakes remain immutable source assets.
use denrim_materials::{geometry::Texture, gpu::*, integration::InstanceRecord};
use renderkit::{
    GpuContext, GpuImage, ImageDescriptor, StatusReadback, TargetDescriptor, TargetLease,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use theframework::prelude::ThePalette;
#[cfg(target_arch = "wasm32")]
mod browser;

fn procedural_source(spec: &Texture) -> bool {
    !spec.source.starts_with("ruleset:material/")
        && !spec.source.starts_with("ruleset:icon/")
        && theframework::prelude::Uuid::parse_str(&spec.source)
            .ok()
            .and_then(|id| {
                shared::ruleset_materials::resolution(shared::rulesets::DEFAULT_RULESET_CONFIG, &id)
            })
            .is_none()
}

const ATLAS_BUDGET: u64 = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Stats {
    pub builds: u64,
    pub hits: u64,
    pub procedural_sources: u64,
    pub source_upload_bytes: u64,
    pub diagnostic_readback_bytes: u64,
}
struct Atlas {
    key: String,
    submitted: bool,
    failure: Option<String>,
    maps: Maps,
}
/// Keep this snapshot alive through submission of its consuming encoder.
#[must_use = "retain atlas maps through command submission"]
#[derive(Clone)]
pub(super) struct Maps {
    pub sampled: [wgpu::TextureView; 3],
    _leases: [TargetLease; 3],
    // Keep encoded float maps and scratch alive across producer cancellation.
    _work: Vec<Arc<RenderTicket>>,
}
struct DiagnosticMap {
    ticket: Arc<RenderTicket>,
    reader: StatusReadback,
}
pub(super) struct GpuTextures {
    context: Arc<GpuContext>,
    metadata: super::textures::Cache,
    runtime: Option<Arc<MaterialZ>>,
    #[cfg(target_arch = "wasm32")]
    browser: browser::Compiler,
    instances: HashMap<String, MaterialInstance>,
    layout: wgpu::BindGroupLayout,
    conversion: wgpu::ComputePipeline,
    initialization: wgpu::ComputePipeline,
    dummy: Arc<GpuImage>,
    atlas: Option<Atlas>,
    queued: Vec<Arc<RenderTicket>>,
    mapping: Vec<DiagnosticMap>,
    pub stats: Stats,
}
impl GpuTextures {
    pub fn new(context: Arc<GpuContext>) -> Result<Self, String> {
        let device = &context.device;
        let mut entries = Vec::new();
        for binding in 0..5 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        for binding in 5..8 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 8,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("geometry material conversion"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("geometry material conversion"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("geometry material conversion"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_textures.wgsl").into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("geometry material conversion"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let conversion = pipeline("convert");
        let initialization = pipeline("initialize");
        let dummy = GpuImage::new(
            &context,
            ImageDescriptor {
                format: wgpu::TextureFormat::Rgba16Float,
                color_space: renderkit::ColorSpace::Linear,
                ..ImageDescriptor::rgba(1, 1)
            },
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            queued: Vec::new(),
            mapping: Vec::new(),
            context,
            metadata: Default::default(),
            runtime: None,
            #[cfg(target_arch = "wasm32")]
            browser: Default::default(),
            instances: Default::default(),
            layout,
            conversion,
            initialization,
            dummy,
            atlas: None,
            stats: Default::default(),
        })
    }
    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        inputs: [&wgpu::TextureView; 5],
        outputs: &[wgpu::TextureView; 3],
        parameters: [u32; 4],
        initialize: bool,
    ) -> Result<(), String> {
        let bytes: Vec<_> = parameters.into_iter().flat_map(u32::to_ne_bytes).collect();
        let uniform = self
            .context
            .upload_immutable_buffer(&bytes, wgpu::BufferUsages::UNIFORM)
            .map_err(|e| e.to_string())?;
        let mut entries: Vec<_> = inputs
            .into_iter()
            .enumerate()
            .map(|(binding, view)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.extend(
            outputs
                .iter()
                .enumerate()
                .map(|(i, view)| wgpu::BindGroupEntry {
                    binding: i as u32 + 5,
                    resource: wgpu::BindingResource::TextureView(view),
                }),
        );
        entries.push(wgpu::BindGroupEntry {
            binding: 8,
            resource: uniform.buffer().as_entire_binding(),
        });
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("geometry atlas layer"),
                layout: &self.layout,
                entries: &entries,
            });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("geometry atlas conversion"),
            timestamp_writes: None,
        });
        pass.set_pipeline(if initialize {
            &self.initialization
        } else {
            &self.conversion
        });
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(parameters[0].div_ceil(8), parameters[1].div_ceil(8), 1);
        Ok(())
    }
    /// Encoding can be cancelled by the caller. Reuse only after the encoder
    /// containing this atlas's conversion has actually been submitted.
    pub fn submitted(&mut self) {
        if let Some(atlas) = &mut self.atlas {
            atlas.submitted = true;
        }
        for mut ticket in self.queued.drain(..) {
            if let Some(ticket) = Arc::get_mut(&mut ticket) {
                ticket.submitted();
            }
            match StatusReadback::map(&self.context, ticket.readback.clone()) {
                Ok(reader) => {
                    self.mapping.push(DiagnosticMap { ticket, reader });
                    self.stats.diagnostic_readback_bytes += 4;
                }
                Err(error) => {
                    if let Some(atlas) = &mut self.atlas {
                        atlas.failure = Some(error.to_string());
                    }
                }
            }
        }
    }
    fn poll_diagnostics(&mut self) {
        let mut index = 0;
        while index < self.mapping.len() {
            let result = match self.mapping[index].reader.try_finish(&self.context) {
                Ok(None) => {
                    index += 1;
                    continue;
                }
                Ok(Some(flags)) => self.mapping[index]
                    .ticket
                    .check_diagnostics(flags)
                    .map_err(|e| e.to_string()),
                Err(error) => Err(error.to_string()),
            };
            self.mapping.swap_remove(index);
            if let Err(error) = result {
                if let Some(atlas) = &mut self.atlas {
                    atlas.failure = Some(error);
                }
            }
        }
    }
    /// Only bounded four-byte status buffers are mapped asynchronously. A new
    /// source cancels old receipts; delayed old callbacks cannot poison it.
    pub fn status(&mut self) -> Result<bool, String> {
        self.poll_diagnostics();
        let Some(atlas) = &self.atlas else {
            return Ok(false);
        };
        if let Some(error) = &atlas.failure {
            return Err(error.clone());
        }
        Ok(atlas.submitted && self.mapping.is_empty())
    }
    fn cancel_diagnostics(&mut self) {
        if !self.mapping.is_empty() {
            if let Some(atlas) = &mut self.atlas {
                atlas.submitted = false;
            }
        }
        // Each reader owns its mailbox and cancels/unmaps on drop. Old source
        // callbacks cannot address the replacement atlas or any new reader.
        self.mapping.clear();
        self.queued.clear();
    }
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        specs: &[&Texture],
        palette: &ThePalette,
        seconds: f32,
    ) -> Result<Option<Maps>, String> {
        self.poll_diagnostics();
        if specs.len() > 16
            || specs.iter().any(|s| {
                s.size == 0 || s.size > self.context.device.limits().max_texture_dimension_2d
            })
        {
            return Err("Invalid geometry material texture sizes/count".into());
        }
        let size = specs.iter().map(|s| s.size).max().unwrap_or(16);
        let layers = specs.len() as u32 + 1;
        let bytes = size as u64 * size as u64 * layers as u64 * 12;
        if bytes > ATLAS_BUDGET {
            return Err("Geometry material atlas exceeds its 64 MiB budget".into());
        }
        #[cfg(target_arch = "wasm32")]
        {
            let sources = specs
                .iter()
                .filter(|spec| procedural_source(spec))
                .map(|spec| browser::Source {
                    key: format!("{spec:?}"),
                    address: format!("{spec:?}:{:?}", palette.colors),
                    spec,
                })
                .collect();
            if !self.browser.prepare(
                self.context.clone(),
                &mut self.runtime,
                &mut self.instances,
                sources,
                palette,
            )? {
                return Ok(None);
            }
        }
        let frames = specs
            .iter()
            .map(|s| self.metadata.frame(s, palette, seconds))
            .collect::<Result<Vec<_>, _>>()?;
        let key = format!("{specs:?}:{frames:?}:{:?}", palette.colors);
        if let Some(atlas) = self.atlas.as_ref().filter(|a| a.submitted && a.key == key) {
            if let Some(error) = &atlas.failure {
                return Err(error.clone());
            }
            self.stats.hits += 1;
            return Ok(Some(atlas.maps.clone()));
        }
        self.cancel_diagnostics();
        let mut diagnostics = Vec::new();
        let allocate = || {
            let mut target = TargetDescriptor::color(size, size, wgpu::TextureFormat::Rgba8Unorm);
            target.array_layers = layers;
            target.usage = wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC;
            self.context
                .acquire_target(target)
                .map_err(|e| e.to_string())
        };
        // Acquire the entire replacement while the old atlas is still pinned.
        let leases = [allocate()?, allocate()?, allocate()?];
        let outputs = std::array::from_fn(|i| {
            leases[i]
                .texture()
                .create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
                    ..Default::default()
                })
        });
        let sampled = std::array::from_fn(|i| {
            leases[i]
                .texture()
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(if i == 1 {
                        wgpu::TextureFormat::Rgba8Unorm
                    } else {
                        wgpu::TextureFormat::Rgba8UnormSrgb
                    }),
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
                    ..Default::default()
                })
        });
        self.dispatch(
            encoder,
            [&self.dummy.view; 5],
            &outputs,
            [size, size, 0, 1],
            true,
        )?;
        let active: HashSet<_> = specs.iter().map(|s| format!("{s:?}")).collect();
        self.instances.retain(|key, _| active.contains(key));
        let mut uploaded = 0;
        let mut procedural = 0;
        for (i, (spec, (_, phase))) in specs.iter().zip(frames).enumerate() {
            let layer = i as u32 + 1;
            // These caches are authoritative CPU source assets, as before the
            // rewrite. Do not retain every animation frame's CPU pixel arrays.
            if !procedural_source(spec) {
                let mut source = super::textures::Cache::default();
                let maps = source.image(
                    spec,
                    palette,
                    &self.context.device,
                    &self.context.queue,
                    seconds,
                )?;
                for (channel, map) in [&maps.color, &maps.data, &maps.emission]
                    .into_iter()
                    .enumerate()
                {
                    let mut resized = Vec::with_capacity((size * size * 4) as usize);
                    for y in 0..size {
                        for x in 0..size {
                            let at = (((y * spec.size / size) * spec.size + x * spec.size / size)
                                * 4) as usize;
                            resized.extend_from_slice(&map[at..at + 4]);
                        }
                    }
                    self.context.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: leases[channel].texture(),
                            mip_level: 0,
                            origin: wgpu::Origin3d {
                                x: 0,
                                y: 0,
                                z: layer,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &resized,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(size * 4),
                            rows_per_image: Some(size),
                        },
                        wgpu::Extent3d {
                            width: size,
                            height: size,
                            depth_or_array_layers: 1,
                        },
                    );
                    self.context.record_upload(resized.len() as u64);
                    uploaded += resized.len() as u64;
                }
            } else {
                let key = format!("{spec:?}");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let asset = super::textures::asset(spec, palette, None)?;
                    if self.runtime.is_none() {
                        self.runtime = Some(Arc::new(
                            pollster::block_on(MaterialZ::new_with_context(self.context.clone()))
                                .map_err(|e| e.to_string())?,
                        ));
                    }
                    let runtime = self.runtime.as_ref().unwrap();
                    if !self
                        .instances
                        .get_mut(&key)
                        .is_some_and(|instance| instance.update_asset(&asset).is_ok())
                    {
                        let instance = pollster::block_on(
                            runtime.compile(&asset, InstanceRecord::new("native-geometry-texture")),
                        )
                        .map_err(|e| e.to_string())?;
                        self.instances.insert(key.clone(), instance);
                    }
                }
                let runtime = self
                    .runtime
                    .as_ref()
                    .ok_or("Geometry material runtime is not prepared")?;
                let instance = self
                    .instances
                    .get(&key)
                    .ok_or("Geometry material source is not prepared")?;
                let request = RenderRequest {
                    time: phase,
                    resolution: [spec.size; 2],
                    domain: SurfaceDomain::rect([-0.5; 2], [1.; 2]),
                };
                let surface = runtime
                    .create_surface(&request)
                    .map_err(|e| e.to_string())?;
                let diagnostic = runtime
                    .encode(encoder, instance, &request, &surface)
                    .map_err(|e| e.to_string())?;
                diagnostics.push(Arc::new(diagnostic));
                self.dispatch(
                    encoder,
                    std::array::from_fn(|i| &surface.maps[i].view),
                    &outputs,
                    [size, size, layer, spec.size],
                    false,
                )?;
                procedural += 1;
            }
        }
        self.stats.builds += 1;
        self.stats.procedural_sources += procedural;
        self.stats.source_upload_bytes += uploaded;
        self.queued = diagnostics;
        let maps = Maps {
            sampled,
            _leases: leases,
            _work: Vec::new(),
        };
        self.atlas = Some(Atlas {
            key,
            submitted: false,
            failure: None,
            maps: maps.clone(),
        });
        let mut maps = maps;
        maps._work = self.queued.clone();
        Ok(Some(maps))
    }
}

impl Drop for GpuTextures {
    fn drop(&mut self) {
        self.cancel_diagnostics();
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use denrim_materials::integration::EditableMaterialAsset;
    use renderkit::{GpuImage, ImageDescriptor, ReadbackTicket};
    pub(super) fn capture(
        context: &GpuContext,
        atlas: &Atlas,
        channel: usize,
        layer: u32,
    ) -> Vec<u8> {
        let source = atlas.maps._leases[channel].texture();
        let image = GpuImage::new(
            context,
            ImageDescriptor {
                format: wgpu::TextureFormat::Rgba8Unorm,
                ..ImageDescriptor::rgba(source.width(), source.height())
            },
        )
        .unwrap();
        let mut encoder = context.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: source,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            image.texture.as_image_copy(),
            image.texture.size(),
        );
        context.queue.submit([encoder.finish()]);
        let mut capture = ReadbackTicket::capture(context, image).unwrap();
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        capture
            .try_finish(context)
            .unwrap()
            .unwrap()
            .pixels()
            .to_vec()
    }
    pub fn material(size: u32) -> Texture {
        let mut asset = EditableMaterialAsset::parse(
            "native-geometry-fixture",
            r#"project 1
material "surface" called "Geometry atlas fixture"
  mode pbr
  tiling raw
  row "noise"
    use "Perlin Noise" id "noise"
  row "color"
    use "Mix" id "pigment"
      set "color1" to 0.08 0.16 0.025 1
      set "color2" to 0.4 0.6 0.15 1
      set "factor" to 0.35 0.35 0.35
  output base color from row "color" xyz
  output roughness from row "noise" x
  output opacity from row "noise" x
  output emission from row "color" xyz
"#,
            "surface",
        )
        .unwrap();
        asset.root.project.materials[0].animation.frames = 3;
        asset.root.project.materials[0].animation.duration = 2.;
        Texture {
            source: format!("native-material-{size}"),
            bundle: Some(asset.save().unwrap()),
            size,
        }
    }
    #[test]
    #[ignore = "requires an actual native GPU"]
    fn native_geometry_atlas_matches_reference_maps_without_mapping_displayed_sources() {
        let context = pollster::block_on(GpuContext::headless()).unwrap();
        let mut cache = GpuTextures::new(context.clone()).unwrap();
        let palette = ThePalette::default();
        let specs = [material(16), material(32)];
        // A cancelled outer frame must not make an unsubmitted conversion a
        // cache hit in the next frame.
        let mut cancelled = context.device.create_command_encoder(&Default::default());
        cache
            .encode(
                &mut cancelled,
                &specs.iter().collect::<Vec<_>>(),
                &palette,
                0.,
            )
            .unwrap();
        drop(cancelled);
        assert!(!cache.atlas.as_ref().unwrap().submitted);
        for seconds in [0., 0.8, 2.1] {
            let mut encoder = context.device.create_command_encoder(&Default::default());
            cache
                .encode(
                    &mut encoder,
                    &specs.iter().collect::<Vec<_>>(),
                    &palette,
                    seconds,
                )
                .unwrap();
            context.queue.submit([encoder.finish()]);
            cache.submitted();
            assert_eq!(
                cache.stats.source_upload_bytes, 0,
                "procedural maps must not roundtrip through CPU pixels"
            );
            let before = cache.stats;
            let mut encoder = context.device.create_command_encoder(&Default::default());
            cache
                .encode(
                    &mut encoder,
                    &specs.iter().collect::<Vec<_>>(),
                    &palette,
                    seconds,
                )
                .unwrap();
            context.queue.submit([encoder.finish()]);
            assert_eq!(cache.stats.builds, before.builds);
            assert_eq!(cache.stats.procedural_sources, before.procedural_sources);
            assert_eq!(cache.stats.hits, before.hits + 1);
            // Captures below are explicit comparison consumers. The reference
            // still performs its old readbacks; it is not a display producer.
            for (i, spec) in specs.iter().enumerate() {
                let mut reference = super::super::textures::Cache::default();
                let maps = reference
                    .image(spec, &palette, &context.device, &context.queue, seconds)
                    .unwrap();
                for (channel, pixels) in [&maps.color, &maps.data, &maps.emission]
                    .into_iter()
                    .enumerate()
                {
                    let expected = (0..32)
                        .flat_map(|y| {
                            (0..32).flat_map(move |x| {
                                let at = (((y * spec.size / 32) * spec.size + x * spec.size / 32)
                                    * 4) as usize;
                                pixels[at..at + 4].iter().copied()
                            })
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        capture(
                            &context,
                            cache.atlas.as_ref().unwrap(),
                            channel,
                            i as u32 + 1
                        ),
                        expected,
                        "seconds={seconds}, size={}, channel={channel}",
                        spec.size
                    );
                }
            }
        }
        let before = cache.stats;
        let mut invalid = material(32);
        invalid.size = 0;
        let mut encoder = context.device.create_command_encoder(&Default::default());
        assert!(
            cache
                .encode(&mut encoder, &[&invalid], &palette, 0.)
                .is_err()
        );
        assert_eq!(cache.stats, before);
        let mut oversized = material(8192);
        assert!(
            cache
                .encode(&mut encoder, &[&oversized], &palette, 0.)
                .is_err()
        );
        oversized.size = 32;
        oversized.source = "missing".into();
        oversized.bundle = None;
        assert!(
            cache
                .encode(&mut encoder, &[&oversized], &palette, 0.)
                .is_err()
        );
        assert_eq!(
            cache.stats, before,
            "failed source replacement leaves the published atlas intact"
        );
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::tests::capture;
    use super::*;
    use wgpu::util::DeviceExt;
    fn poll(context: &GpuContext) {
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
    }
    #[test]
    #[ignore = "native GPU; atlas snapshots pin all planes before submission"]
    fn native_geometry_atlas_snapshots_pin_pending_consumers_and_retire() {
        let context = pollster::block_on(GpuContext::headless()).unwrap();
        let mut cache = GpuTextures::new(context.clone()).unwrap();
        let palette = ThePalette::default();
        let mut first_encoder = context.device.create_command_encoder(&Default::default());
        let first = cache
            .encode(&mut first_encoder, &[], &palette, 0.)
            .unwrap()
            .unwrap();
        let mut second_encoder = context.device.create_command_encoder(&Default::default());
        let second = cache
            .encode(&mut second_encoder, &[], &palette, 0.)
            .unwrap()
            .unwrap();
        assert_eq!(
            context.target_pool_usage().allocations,
            6,
            "unsubmitted first snapshot cannot be reused"
        );
        assert_eq!(context.target_pool_usage().hits, 0);
        drop(cache);
        context.set_target_cache_limits(0, 0);
        assert_eq!(context.target_pool_usage().live_bytes, 16 * 16 * 12 * 2);
        assert_eq!(context.target_pool_usage().live_allocations, 6);
        context
            .queue
            .submit([second_encoder.finish(), first_encoder.finish()]);
        let first_atlas = Atlas {
            key: String::new(),
            submitted: true,
            failure: None,
            maps: first,
        };
        let second_atlas = Atlas {
            key: String::new(),
            submitted: true,
            failure: None,
            maps: second,
        };
        for atlas in [&first_atlas, &second_atlas] {
            assert_eq!(capture(&context, atlas, 0, 0), [255; 4].repeat(16 * 16));
            assert_eq!(capture(&context, atlas, 1, 0), [255; 4].repeat(16 * 16));
            assert_eq!(capture(&context, atlas, 2, 0), [0; 4].repeat(16 * 16));
        }
        drop(first_atlas);
        assert_eq!(context.target_pool_usage().live_bytes, 16 * 16 * 12);
        drop(second_atlas);
        assert_eq!(context.target_pool_usage().live_bytes, 0);
        assert_eq!(context.target_pool_usage().live_allocations, 0);
    }

    #[test]
    #[ignore = "native GPU; procedural work snapshots survive queue replacement"]
    fn native_geometry_procedural_work_is_owned_by_pending_consumer_snapshots() {
        let context = pollster::block_on(GpuContext::headless()).unwrap();
        let mut cache = GpuTextures::new(context.clone()).unwrap();
        let palette = ThePalette::default();
        let sources = [super::tests::material(16), super::tests::material(32)];
        let mut first_encoder = context.device.create_command_encoder(&Default::default());
        let first = cache
            .encode(&mut first_encoder, &[&sources[0]], &palette, 0.)
            .unwrap()
            .unwrap();
        let mut second_encoder = context.device.create_command_encoder(&Default::default());
        let second = cache
            .encode(&mut second_encoder, &[&sources[1]], &palette, 0.8)
            .unwrap()
            .unwrap();
        assert_eq!(first._work.len(), 1);
        assert_eq!(second._work.len(), 1);
        drop(cache);
        context.set_target_cache_limits(0, 0);
        context.set_working_buffer_cache_limits(0, 0);
        context.set_immutable_buffer_cache_limits(0, 0);
        assert!(context.target_pool_usage().live_bytes > 0);
        assert!(context.working_buffer_pool_usage().live_bytes > 0);
        assert!(context.immutable_buffer_cache_usage().live_bytes > 0);
        context
            .queue
            .submit([second_encoder.finish(), first_encoder.finish()]);
        let first = Atlas {
            key: String::new(),
            submitted: true,
            failure: None,
            maps: first,
        };
        let second = Atlas {
            key: String::new(),
            submitted: true,
            failure: None,
            maps: second,
        };
        for ((atlas, spec), seconds) in [&first, &second].into_iter().zip(&sources).zip([0., 0.8]) {
            let mut reference = super::super::textures::Cache::default();
            let expected = reference
                .image(spec, &palette, &context.device, &context.queue, seconds)
                .unwrap();
            for (channel, pixels) in [&expected.color, &expected.data, &expected.emission]
                .into_iter()
                .enumerate()
            {
                assert_eq!(capture(&context, atlas, channel, 1), *pixels);
            }
        }
        drop(first);
        assert!(context.working_buffer_pool_usage().live_bytes > 0);
        drop(second);
        assert_eq!(context.target_pool_usage().live_bytes, 0);
        assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
        assert_eq!(context.immutable_buffer_cache_usage().live_bytes, 0);
    }

    #[test]
    #[ignore = "requires an actual native GPU"]
    fn native_geometry_diagnostics_reject_failed_sources_cancel_stale_maps_and_recover() {
        let context = pollster::block_on(GpuContext::headless()).unwrap();
        let mut cache = GpuTextures::new(context.clone()).unwrap();
        let palette = ThePalette::default();
        let source = super::tests::material(16);
        let mut encoder = context.device.create_command_encoder(&Default::default());
        cache
            .encode(&mut encoder, &[&source], &palette, 0.)
            .unwrap();
        // Inject a real GPU status flag after the MaterialZ pass. This exercises
        // its exact diagnostic decoder and the adapter's asynchronous ownership.
        let failure = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("test failed material status"),
                contents: &1_u32.to_le_bytes(),
                usage: wgpu::BufferUsages::COPY_SRC,
            });
        encoder.copy_buffer_to_buffer(&failure, 0, &cache.queued[0].readback, 0, 4);
        context.queue.submit([encoder.finish()]);
        cache.submitted();
        assert_eq!(cache.stats.diagnostic_readback_bytes, 4);
        poll(&context);
        assert!(cache.status().unwrap_err().contains("channel flags 0x1"));
        let before = cache.stats;
        let mut rejected = context.device.create_command_encoder(&Default::default());
        assert!(
            cache
                .encode(&mut rejected, &[&source], &palette, 0.)
                .is_err()
        );
        assert_eq!(
            cache.stats, before,
            "an invalid source must not be reused as a ready atlas"
        );
        let replacement = super::tests::material(32);
        let mut next = context.device.create_command_encoder(&Default::default());
        cache
            .encode(&mut next, &[&replacement], &palette, 0.)
            .unwrap();
        context.queue.submit([next.finish()]);
        cache.submitted();
        assert!(!cache.mapping.is_empty());
        let mut newer = context.device.create_command_encoder(&Default::default());
        cache
            .encode(&mut newer, &[&replacement], &palette, 0.8)
            .unwrap();
        context.queue.submit([newer.finish()]);
        cache.submitted();
        poll(&context);
        assert!(cache.status().unwrap());
        assert!(cache.mapping.is_empty());
        assert_eq!(cache.stats.diagnostic_readback_bytes, 12);
        assert_eq!(context.transfers().status_readback_bytes, 12);
        assert_eq!(
            context.transfers().readback_bytes,
            0,
            "only four-byte status metadata is mapped, never displayed material/color pixels"
        );
        // Closing while a completion is still pending must not leave a callback
        // that tries to read an already-unmapped buffer.
        let mut closing = context.device.create_command_encoder(&Default::default());
        cache
            .encode(&mut closing, &[&source], &palette, 0.)
            .unwrap();
        context.queue.submit([closing.finish()]);
        cache.submitted();
        drop(cache);
        poll(&context);
    }
}
