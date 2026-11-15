//! Direct MaterialZ display. Float maps, quantization, viewport sampling and
//! checker/alpha composition remain on the injected GPU; only status is mapped.
use denrim_materials::{
    gpu::*,
    integration::{EditableMaterialAsset, InstanceRecord},
};
use renderkit::{AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, StatusReadback};
use std::sync::Arc;
const PREVIEW_SIZE: u32 = 384;
const TARGET_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Clone)]
pub(super) struct Request {
    pub asset: Arc<EditableMaterialAsset>,
    pub time: f32,
    /// The root material owns the preview domain, including transient selections.
    pub origin: Option<[f32; 2]>,
    pub pixels: u32,
    pub continuous: bool,
    /// Runtime map order: Color/Opacity/Roughness/Metallic/Emission.
    pub channel: u32,
    pub size: [u32; 2],
    pub checker: [[u8; 4]; 2],
}
pub(super) struct Frame {
    pub image: Arc<GpuImage>,
    pub size: [u32; 2],
    pub time: f32,
}
impl Frame {
    pub fn paint(&self, image: renderkit::ImageHandle) -> theframework::paint::PaintList {
        let mut list = theframework::paint::PaintList::default();
        list.image(
            image,
            theframework::paint::Rect::new(0., 0., self.size[0] as f32, self.size[1] as f32),
            theframework::paint::Sampling::Nearest,
        );
        list
    }
}
struct Grid {
    resolution: [u32; 2],
    origin: [f32; 2],
    span: [f32; 2],
    density: f32,
    first: [f32; 2],
}
impl Grid {
    fn new(origin: [f32; 2], span: [f32; 2], pixels: u32, source_size: u32) -> Self {
        if pixels == 0 {
            return Self {
                resolution: [source_size; 2],
                origin,
                span,
                density: 0.,
                first: [0.; 2],
            };
        }
        let density =
            (pixels as f32).min((source_size as f32 / span[0]).min(source_size as f32 / span[1]));
        let first = origin.map(|v| (v * density).floor());
        let resolution = std::array::from_fn(|i| {
            (((origin[i] + span[i]) * density).ceil() - first[i]).max(1.) as u32
        });
        Self {
            resolution,
            origin: first.map(|v| v / density),
            span: resolution.map(|v| v as f32 / density),
            density,
            first,
        }
    }
}
struct Evaluation {
    asset: Arc<EditableMaterialAsset>,
    time: u32,
    pixels: u32,
    span: [f32; 2],
    origin: [f32; 2],
    grid: Grid,
    surface: Surface,
    ticket: RenderTicket,
    status: Option<StatusReadback>,
    failure: Option<String>,
    valid: bool,
}
impl Evaluation {
    fn same(&self, request: &Request, span: [f32; 2], origin: [f32; 2]) -> bool {
        self.time == request.time.to_bits() && self.same_source(request, span, origin)
    }
    fn same_source(&self, request: &Request, span: [f32; 2], origin: [f32; 2]) -> bool {
        Arc::ptr_eq(&self.asset, &request.asset)
            && self.pixels == request.pixels
            && self.span == span
            && self.origin == origin
    }
    fn ready(&mut self, context: &GpuContext) -> Result<bool, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if let Some(status) = &mut self.status {
            let result = match status.try_finish(context) {
                Ok(None) => return Ok(false),
                Ok(Some(flags)) => self
                    .ticket
                    .check_diagnostics(flags)
                    .map_err(|e| e.to_string()),
                Err(error) => Err(error.to_string()),
            };
            self.status = None;
            if let Err(error) = result {
                self.failure = Some(error.clone());
                return Err(error);
            }
            self.valid = true;
        }
        Ok(self.valid)
    }
}
pub(super) struct Preview {
    context: Arc<GpuContext>,
    runtime: Option<Arc<MaterialZ>>,
    #[cfg(target_arch = "wasm32")]
    initializing: Option<std::sync::mpsc::Receiver<Result<Arc<MaterialZ>, String>>>,
    #[cfg(target_arch = "wasm32")]
    compiling: Option<(
        Arc<EditableMaterialAsset>,
        std::sync::mpsc::Receiver<Result<MaterialInstance, String>>,
    )>,
    compiled: Option<MaterialInstance>,
    compiled_asset: Option<Arc<EditableMaterialAsset>>,
    evaluation: Option<Evaluation>,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    targets: Vec<Arc<GpuImage>>,
    thumbnail: bool,
    pub evaluations: u64,
}
impl Preview {
    pub fn new(context: Arc<GpuContext>) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = Some(Arc::new(
            pollster::block_on(MaterialZ::new_with_context(context.clone()))
                .map_err(|e| e.to_string())?,
        ));
        #[cfg(target_arch = "wasm32")]
        let runtime = None;
        #[cfg(target_arch = "wasm32")]
        let initializing = {
            let (done, receive) = std::sync::mpsc::channel();
            let context = context.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = MaterialZ::new_with_context(context.clone())
                    .await
                    .map(Arc::new)
                    .map_err(|e| e.to_string());
                let _ = done.send(result);
            });
            Some(receive)
        };
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for binding in 1..=5 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("native material display"),
                entries: &entries,
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("native material display"),
                source: wgpu::ShaderSource::Wgsl(include_str!("native_preview.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("native material display"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("native material display"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
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
        Ok(Self {
            context,
            runtime,
            #[cfg(target_arch = "wasm32")]
            initializing,
            #[cfg(target_arch = "wasm32")]
            compiling: None,
            compiled: None,
            compiled_asset: None,
            evaluation: None,
            layout,
            pipeline,
            targets: Vec::new(),
            thumbnail: false,
            evaluations: 0,
        })
    }
    pub fn thumbnails(context: Arc<GpuContext>) -> Result<Self, String> {
        let mut producer = Self::new(context)?;
        producer.thumbnail = true;
        Ok(producer)
    }
    /// Transfer an immutable result to the bounded graph cache. It is no longer
    /// eligible for this producer's viewport target reuse or retention budget.
    pub fn transfer_output(&mut self, image: &Arc<GpuImage>) {
        self.targets.retain(|target| !Arc::ptr_eq(target, image));
    }
    pub fn generation(&self) -> u64 {
        self.context.generation()
    }
    fn runtime(&mut self) -> Result<Option<Arc<MaterialZ>>, String> {
        #[cfg(target_arch = "wasm32")]
        if self.runtime.is_none() {
            let receive = self
                .initializing
                .as_ref()
                .ok_or("Material runtime initialization was retired")?;
            match receive.try_recv() {
                Ok(result) => {
                    self.initializing = None;
                    self.runtime = Some(result?);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(None),
                Err(_) => return Err("Material runtime initialization was cancelled".into()),
            }
        }
        Ok(self.runtime.clone())
    }
    fn compile(
        &mut self,
        runtime: Arc<MaterialZ>,
        asset: &Arc<EditableMaterialAsset>,
    ) -> Result<bool, String> {
        if self
            .compiled_asset
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, asset))
        {
            return Ok(true);
        }
        #[cfg(target_arch = "wasm32")]
        if let Some((pending, receive)) = &self.compiling {
            match receive.try_recv() {
                Ok(result) => {
                    let same = Arc::ptr_eq(pending, asset);
                    let pending = pending.clone();
                    self.compiling = None;
                    if same {
                        self.compiled = Some(result?);
                        self.compiled_asset = Some(pending);
                        return Ok(true);
                    }
                    // A newer source superseded this job. Its result/error may
                    // not publish against the current request's identity.
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(false),
                Err(_) => {
                    self.compiling = None;
                    return Err("Material compile was cancelled".into());
                }
            }
        }
        if self
            .compiled
            .as_mut()
            .is_some_and(|instance| instance.update_asset(asset).is_ok())
        {
            self.compiled_asset = Some(asset.clone());
            return Ok(true);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.compiled = Some(
                pollster::block_on(runtime.compile(asset, InstanceRecord::new("eldiron-preview")))
                    .map_err(|e| e.to_string())?,
            );
            self.compiled_asset = Some(asset.clone());
            Ok(true)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (done, receive) = std::sync::mpsc::channel();
            let source = asset.clone();
            self.compiling = Some((source.clone(), receive));
            wasm_bindgen_futures::spawn_local(async move {
                let result = runtime
                    .compile(&source, InstanceRecord::new("eldiron-preview"))
                    .await
                    .map_err(|e| e.to_string());
                let _ = done.send(result);
            });
            Ok(false)
        }
    }
    pub fn render(&mut self, request: &Request, density: f32) -> Result<Option<Frame>, String> {
        if self.context.is_lost() {
            return Err("Material preview device was lost".into());
        }
        if request.size.contains(&0)
            || !request.time.is_finite()
            || !density.is_finite()
            || density <= 0.
            || request.channel > 4
        {
            return Err("Invalid material preview size/time/channel".into());
        }
        let physical = request.size.map(|n| (n as f64 * density as f64).round());
        if physical
            .iter()
            .any(|v| *v < 1. || *v > self.context.device.limits().max_texture_dimension_2d as f64)
        {
            return Err("Material preview exceeds GPU texture limits".into());
        }
        let [w, h] = physical.map(|v| v as u32);
        let bytes = w as u64 * h as u64 * 4;
        if bytes > TARGET_BYTES {
            return Err("Material preview exceeds its output budget".into());
        }
        let source_size = if self.thumbnail { 64 } else { PREVIEW_SIZE };
        let span = if self.thumbnail {
            [1.; 2]
        } else if request.continuous {
            [
                2. * (request.size[0] as f32 / request.size[1] as f32).max(0.1),
                2.,
            ]
        } else {
            [1.; 2]
        };
        let origin = request.origin.unwrap_or_else(|| {
            request
                .asset
                .instances
                .iter()
                .find(|r| r.id == super::composition::SURFACE_DOMAIN)
                .map_or([-0.5; 2], |r| [r.uv_transform[4], r.uv_transform[5]])
        });
        if origin.iter().chain(&span).any(|v| !v.is_finite()) {
            return Err("Invalid material preview domain".into());
        }
        // Animation can advance before a low-FPS host observes this status.
        // Publish the completed compatible source once with its own time stamp.
        let compatible_pending = self.evaluation.as_ref().is_some_and(|e| {
            !e.valid && e.failure.is_none() && e.same_source(request, span, origin)
        });
        let completed_older = if compatible_pending {
            if !self.evaluation.as_mut().unwrap().ready(&self.context)? {
                return Ok(None);
            }
            true
        } else {
            false
        };
        if !completed_older
            && self
                .evaluation
                .as_ref()
                .is_none_or(|e| !e.same(request, span, origin))
        {
            let Some(runtime) = self.runtime()? else {
                return Ok(None);
            };
            if !self.compile(runtime.clone(), &request.asset)? {
                return Ok(None);
            }
            let grid = Grid::new(origin, span, request.pixels, source_size);
            let parameters = RenderRequest {
                time: request.time,
                resolution: grid.resolution,
                domain: SurfaceDomain::rect(grid.origin, grid.span),
            };
            let surface = runtime
                .create_surface(&parameters)
                .map_err(|e| e.to_string())?;
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("native material evaluation"),
                    });
            let mut ticket = runtime
                .encode(
                    &mut encoder,
                    self.compiled.as_ref().unwrap(),
                    &parameters,
                    &surface,
                )
                .map_err(|e| e.to_string())?;
            self.context.queue.submit([encoder.finish()]);
            ticket.submitted();
            let status = StatusReadback::map(&self.context, ticket.readback.clone())
                .map_err(|e| e.to_string())?;
            self.evaluation = Some(Evaluation {
                asset: request.asset.clone(),
                time: request.time.to_bits(),
                pixels: request.pixels,
                span,
                origin,
                grid,
                surface,
                ticket,
                status: Some(status),
                valid: false,
                failure: None,
            });
            self.evaluations += 1;
        }
        let evaluation = self.evaluation.as_mut().unwrap();
        if !evaluation.ready(&self.context)? {
            return Ok(None);
        }
        self.targets.retain(|i| {
            Arc::strong_count(i) > 1 || (i.descriptor.width == w && i.descriptor.height == h)
        });
        let image = if let Some(i) = self.targets.iter().find(|i| {
            Arc::strong_count(i) == 1 && i.descriptor.width == w && i.descriptor.height == h
        }) {
            i.clone()
        } else {
            let resident: u64 = self
                .targets
                .iter()
                .map(|i| i.descriptor.width as u64 * i.descriptor.height as u64 * 4)
                .sum();
            if self.targets.len() >= 3 || resident + bytes > TARGET_BYTES {
                return Ok(None);
            }
            let image = GpuImage::new(
                &self.context,
                ImageDescriptor {
                    width: w,
                    height: h,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    color_space: ColorSpace::Srgb,
                    alpha_mode: AlphaMode::Straight,
                },
            )
            .map_err(|e| e.to_string())?;
            self.targets.push(image.clone());
            image
        };
        let mut data = Vec::new();
        for n in [
            request.size[0] as f32,
            request.size[1] as f32,
            w as f32,
            h as f32,
        ] {
            data.extend(n.to_ne_bytes());
        }
        for n in [
            request.channel,
            u32::from(request.continuous),
            source_size,
            u32::from(self.thumbnail),
        ] {
            data.extend(n.to_ne_bytes());
        }
        for n in [
            origin[0],
            origin[1],
            span[0],
            span[1],
            evaluation.grid.density,
            evaluation.grid.first[0],
            evaluation.grid.first[1],
            0.,
        ] {
            data.extend(n.to_ne_bytes());
        }
        for n in request.checker.into_iter().flatten() {
            data.extend((n as u32).to_ne_bytes());
        }
        let uniform = self
            .context
            .upload_immutable_buffer(&data, wgpu::BufferUsages::UNIFORM)
            .map_err(|e| e.to_string())?;
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.buffer().as_entire_binding(),
        }];
        entries.extend(evaluation.surface.maps.iter().enumerate().map(|(i, map)| {
            wgpu::BindGroupEntry {
                binding: i as u32 + 1,
                resource: wgpu::BindingResource::TextureView(&map.view),
            }
        }));
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("native material display"),
                layout: &self.layout,
                entries: &entries,
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("native material display"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("native material display"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &image.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.draw(0..3, 0..1);
        }
        self.context.queue.submit([encoder.finish()]);
        Ok(Some(Frame {
            image,
            size: request.size,
            time: f32::from_bits(evaluation.time),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::{ImageRegistry, ReadbackTicket};
    use theframework::{prelude::*, render_wgpu::PaintRenderer};
    fn capture(gpu: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
        let mut read = ReadbackTicket::capture(gpu, image).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        read.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
    }
    fn next(preview: &mut Preview, request: &Request, density: f32) -> Frame {
        if let Some(frame) = preview.render(request, density).unwrap() {
            return frame;
        }
        preview
            .context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        preview
            .render(request, density)
            .unwrap()
            .expect("completed status should release the map output")
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_material_maps_match_reference_sampling_channels_style_and_animation() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let runtime = pollster::block_on(MaterialZ::new(&gpu.device, &gpu.queue)).unwrap();
        let source = r#"project 1
material "m" called "Native maps"
  mode pbr
  tiling raw
  animation 3 2
  row "noise"
    use "Perlin Noise" id "noise"
      set "globalScale" to 5
    use "Loop Flow" id "flow"
      set "velocity" to 0 0.35
  output base color from row "noise" xxx
  default opacity to 0.5
  output roughness from row "noise" x
  default metallic to 0.2
  default emission to 0.1 0.2 0.3
"#;
        let asset = Arc::new(EditableMaterialAsset::parse("native-maps", source, "m").unwrap());
        let mut compiled =
            pollster::block_on(runtime.compile(&asset, InstanceRecord::new("eldiron-preview")))
                .unwrap();
        let mut preview = Preview::new(gpu.clone()).unwrap();
        let mut request = Request {
            asset,
            time: 0.,
            origin: None,
            pixels: 16,
            continuous: false,
            channel: 0,
            size: [113, 177],
            checker: [[17, 39, 62, 255], [104, 89, 72, 255]],
        };
        for (pixels, continuous, time, size, origin) in [
            (0, false, 0., [113, 177], [-0.5; 2]),
            (16, false, 0., [113, 177], [-0.5; 2]),
            (32, true, 0., [113, 177], [-0.5; 2]),
            (16, true, 1. / 3., [113, 177], [-0.5; 2]),
            (0, true, 2. / 3., [113, 177], [-0.5; 2]),
            (16, false, 0., [320, 101], [-0.5; 2]),
            (32, true, 0., [16, 320], [-0.5; 2]),
            (16, true, 0.5, [256, 128], [-0.123, 1.8]),
        ] {
            request.size = size;
            if origin != [-0.5; 2] {
                let mut changed = (*request.asset).clone();
                let mut record = InstanceRecord::new(super::super::composition::SURFACE_DOMAIN);
                record.uv_transform[4] = origin[0];
                record.uv_transform[5] = origin[1];
                changed.instances.push(record);
                request.asset = Arc::new(changed);
                compiled = pollster::block_on(
                    runtime.compile(&request.asset, InstanceRecord::new("eldiron-preview")),
                )
                .unwrap();
            }
            request.pixels = pixels;
            request.continuous = continuous;
            request.time = time;
            let span = if continuous {
                [
                    2. * (request.size[0] as f32 / request.size[1] as f32).max(0.1),
                    2.,
                ]
            } else {
                [1.; 2]
            };
            let maps = super::super::gpu::render_styled_domain_time(
                &runtime,
                &compiled,
                &gpu.device,
                &gpu.queue,
                384,
                span,
                origin,
                pixels,
                time,
            )
            .unwrap();
            let images = super::super::gpu::preview_images(&maps);
            for channel in 0..5 {
                let before = gpu.transfers();
                request.channel = channel;
                let frame = next(&mut preview, &request, 1.);
                assert_eq!(
                    gpu.transfers().readback_bytes,
                    before.readback_bytes,
                    "display must not capture maps or viewport colors"
                );
                let actual = capture(&gpu, frame.image.clone());
                let mut reference = TheRGBABuffer::new(TheDim::sized(
                    request.size[0] as i32,
                    request.size[1] as i32,
                ));
                super::super::draw_preview_with_colors(
                    &mut reference,
                    Some((384, &images[channel as usize], continuous)),
                    request.checker,
                );
                if actual != reference.pixels() {
                    let differences = actual
                        .chunks_exact(4)
                        .zip(reference.pixels().chunks_exact(4))
                        .enumerate()
                        .filter(|(_, (a, b))| a != b)
                        .take(8)
                        .collect::<Vec<_>>();
                    panic!(
                        "pixels={pixels} continuous={continuous} time={time} channel={channel}: {differences:?}"
                    );
                }
            }
        }
        assert_eq!(
            preview.evaluations, 8,
            "channel changes must reuse material maps"
        );
        assert_eq!(gpu.transfers().status_readback_bytes, 32);
        let frame = next(&mut preview, &request, 1.);
        let mut images = ImageRegistry::new(gpu.clone());
        let handle = images.insert(frame.image.clone()).unwrap();
        let output = GpuImage::new(
            &gpu,
            ImageDescriptor::rgba(request.size[0], request.size[1]),
        )
        .unwrap();
        let before = gpu.transfers();
        PaintRenderer::new(gpu.clone())
            .unwrap()
            .render(
                &frame.paint(handle),
                &images,
                &output.view,
                output.descriptor.format,
                (request.size[0], request.size[1]),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(gpu.transfers(), before);
        let first = frame;
        let second = next(&mut preview, &request, 1.);
        let third = next(&mut preview, &request, 1.);
        assert!(preview.render(&request, 1.).unwrap().is_none());
        drop(second);
        drop(third);
        let scaled = next(&mut preview, &request, 1.5);
        assert_eq!(
            (
                scaled.image.descriptor.width,
                scaled.image.descriptor.height
            ),
            (384, 192)
        );
        drop(first);
        drop(scaled);
        assert!(preview.render(&request, f32::NAN).is_err());
        // Superseding a submitted source cancels only its own status mailbox.
        request.time = 0.1;
        assert!(preview.render(&request, 1.).unwrap().is_none());
        request.time = 0.2;
        request.pixels = 32;
        assert!(preview.render(&request, 1.).unwrap().is_none());
        let recovered = next(&mut preview, &request, 1.);
        assert_eq!(recovered.size, request.size);
        assert_eq!(recovered.time, request.time);
        drop(recovered);
        request.time = 0.3;
        assert!(preview.render(&request, 1.).unwrap().is_none());
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        request.time = 0.4;
        let older = preview
            .render(&request, 1.)
            .unwrap()
            .expect("animation advance must not starve a completed compatible source");
        assert_eq!(older.time, 0.3);
        let caught_up = next(&mut preview, &request, 1.);
        assert_eq!(caught_up.time, 0.4);
        drop(older);
        drop(caught_up);
        assert_eq!(preview.generation(), gpu.generation());
        let other = pollster::block_on(GpuContext::headless()).unwrap();
        let mut replacement = Preview::new(other.clone()).unwrap();
        let rebuilt = next(&mut replacement, &request, 1.);
        assert_eq!(rebuilt.size, request.size);
        assert_eq!(other.transfers().readback_bytes, 0);
        assert_eq!(other.transfers().status_readback_bytes, 4);
        request.time = 0.6;
        assert!(preview.render(&request, 1.).unwrap().is_none());
        drop(preview);
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
    }
}
