use super::PreviewSelection;
use denrim_materials::{
    gpu::*,
    integration::{EditableMaterialAsset, InstanceRecord},
};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc,
    time::Duration,
};

pub(super) const THUMBNAIL_SIZE: usize = 64;
const RETAINED_THUMBNAIL_PIPELINES: usize = 16;

pub(super) struct Job {
    pub pixels_per_tile: u32,
    pub tile: theframework::prelude::Uuid,
    pub revision: u64,
    pub bundle: String,
    pub continuous: bool,
    pub selection: Option<PreviewSelection>,
    pub preview_aspect: f32,
}
pub(super) struct Completed {
    pub pixels_per_tile: u32,
    pub tile: theframework::prelude::Uuid,
    pub bundle: String,
    pub revision: u64,
    pub frame: Result<Frame, String>,
}
pub(super) struct Frame {
    pub size: usize,
    pub animation_images: Vec<[Vec<u8>; 5]>,
    pub animation_selections: Vec<Vec<u8>>,
    pub images: [Vec<u8>; 5],
    pub tile: rusterix::Texture,
    pub warnings: Vec<String>,
    pub selection: Option<(PreviewSelection, Vec<u8>)>,
    pub continuous: bool,
    /// Unit-domain RGBA thumbnails for every authored operation, keyed by its
    /// stable identity. They do not depend on minimap selection or output wires.
    pub thumbnails: Vec<(String, Vec<u8>)>,
    pub surface_pages: Option<std::sync::Arc<SurfacePages>>,
}

pub(super) struct SurfacePages {
    pub layout: rusterix::TileUvPages,
    pub textures: Vec<rusterix::Texture>,
}

#[derive(Default)]
struct OperationThumbnail {
    bundle: Option<String>,
    image: Vec<u8>,
    compiled: Option<MaterialInstance>,
}
#[derive(Default)]
struct ThumbnailCache {
    asset: Option<(String, String)>,
    bundle: Option<String>,
    operations: HashMap<String, OperationThumbnail>,
}
impl ThumbnailCache {
    fn images(
        &mut self,
        runtime: &MaterialZ,
        asset: &EditableMaterialAsset,
        bundle: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        warnings: &mut Vec<String>,
    ) -> Vec<(String, Vec<u8>)> {
        let identity = (asset.root.id.clone(), asset.selected_material.clone());
        if self.asset.as_ref() != Some(&identity) {
            self.asset = Some(identity);
            self.bundle = None;
            self.operations.clear();
        }
        let operations: Vec<_> = asset
            .root
            .project
            .materials
            .iter()
            .find(|material| material.id == asset.selected_material)
            .into_iter()
            .flat_map(|material| &material.rows)
            .flat_map(|row| &row.tiles)
            .map(|tile| tile.id.clone())
            .collect();
        let active: HashSet<_> = operations.iter().cloned().collect();
        self.operations.retain(|id, _| active.contains(id));
        // Changing only the minimap selection/domain does not need preview AST
        // validation, shader refreshes or GPU readbacks for any graph card.
        if self.bundle.as_deref() == Some(bundle) {
            return operations
                .into_iter()
                .filter_map(|id| {
                    self.operations
                        .get(&id)
                        .map(|entry| (id, entry.image.clone()))
                })
                .collect();
        }
        // Keep bitmap caching for every operation, while bounding the number of
        // retained GPU pipelines on unusually large authored materials.
        let mut overflow_pipeline = None;
        let mut images = Vec::with_capacity(operations.len());
        let operation_count = operations.len();
        for (index, id) in operations.into_iter().enumerate() {
            let result = (|| {
                let preview = super::operation_preview_asset(asset, &id)?;
                let bundle = preview.save().map_err(|e| e.to_string())?;
                let entry = self.operations.entry(id.clone()).or_default();
                if entry.bundle.as_ref() != Some(&bundle) {
                    let compiled = if index < RETAINED_THUMBNAIL_PIPELINES {
                        &mut entry.compiled
                    } else {
                        entry.compiled = None;
                        &mut overflow_pipeline
                    };
                    if !compiled
                        .as_mut()
                        .is_some_and(|instance| instance.update_asset(&preview).is_ok())
                    {
                        *compiled = Some(
                            pollster::block_on(runtime.compile(
                                &preview,
                                InstanceRecord::new("eldiron-operation-thumbnail"),
                            ))
                            .map_err(|e| e.to_string())?,
                        );
                    }
                    let maps = render(
                        runtime,
                        compiled.as_ref().unwrap(),
                        device,
                        queue,
                        THUMBNAIL_SIZE,
                        1.,
                    )?;
                    entry.image = preview_images(&maps)[0].clone();
                    entry.bundle = Some(bundle);
                }
                Ok::<_, String>(entry.image.clone())
            })();
            match result {
                Ok(image) => images.push((id, image)),
                Err(error) => warnings.push(format!("Operation thumbnail {id}: {error}")),
            }
        }
        self.bundle = (images.len() == operation_count).then(|| bundle.to_owned());
        images
    }
}
pub(super) struct Worker {
    jobs: mpsc::Sender<Job>,
    pub results: mpsc::Receiver<Completed>,
}
impl Worker {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let (jobs, rx) = mpsc::channel::<Job>();
        let (tx, results) = mpsc::channel();
        std::thread::Builder::new()
            .name("MaterialZ preview".into())
            .spawn(move || {
                let runtime =
                    pollster::block_on(MaterialZ::new(&device, &queue)).map_err(|e| e.to_string());
                let mut compiled: Option<MaterialInstance> = None;
                let mut selected_compiled: Option<MaterialInstance> = None;
                let mut thumbnail_cache = ThumbnailCache::default();
                let mut page_cache: Option<(
                    theframework::prelude::Uuid,
                    u64,
                    std::sync::Arc<SurfacePages>,
                )> = None;
                let mut pending = std::collections::VecDeque::<Job>::new();
                loop {
                    if pending.is_empty() {
                        match rx.recv() {
                            Ok(job) => pending.push_back(job),
                            Err(_) => break,
                        }
                    }
                    while let Ok(next) = rx.try_recv() {
                        pending.retain(|job| job.tile != next.tile);
                        pending.push_back(next);
                    }
                    let job = pending.pop_front().unwrap();
                    let frame = runtime.as_ref().map_err(Clone::clone).and_then(|runtime| {
                        let asset = EditableMaterialAsset::reopen(&job.bundle)
                            .map_err(|e| e.to_string())?;
                        let reusable = compiled
                            .as_mut()
                            .is_some_and(|instance| instance.update_asset(&asset).is_ok());
                        if !reusable {
                            compiled = Some(
                                pollster::block_on(
                                    runtime.compile(&asset, InstanceRecord::new("eldiron-preview")),
                                )
                                .map_err(|e| e.to_string())?,
                            );
                        }
                        let instance = compiled.as_ref().unwrap();
                        let animation = animation_settings(&asset)?;
                        let mut layout = super::composition::surface_pages::decode(&asset)?;
                        let root = asset.root.project.materials.iter().find(|m|m.id==asset.selected_material).ok_or("Missing material")?;
                        let repeat = (root.tiling.mode != denrim_materials::periodic::Mode::Raw).then_some(root.tiling.period);
                        if let Some(period) = repeat {
                            layout = Some(rusterix::TileUvPages { animation_frames: 1, repeat_period: period, origin: [0;2], size: [1;2], axes: [[0.;3];2], units_per_page: 1 });
                        }

                        if animation.frames > 1 && layout.is_none() {
                            layout = Some(rusterix::TileUvPages {
                                origin: [0, 0],
                                size: [1, 1],
                                axes: [[0.; 3]; 2],
                                units_per_page: 1,
                                animation_frames: 1,
                                repeat_period: [0.; 2],
                            });
                        }
                        let tile_size = super::pixel_style::bake_size(job.pixels_per_tile);
                        let page_count = layout
                            .as_ref()
                            .map_or(1, |p| u64::from(p.size[0]) * u64::from(p.size[1]));
                        let repeat_size = repeat.map(|period| period.map(|v| (v * tile_size as f32).ceil().max(1.) as usize));
                        if repeat_size.is_some_and(|s| s.iter().any(|v| *v > 8192)) { return Err("Repeating material period exceeds the 8192 pixel texture limit".into()); }
                        let texels = repeat_size.map_or(tile_size * tile_size, |s| s[0] * s[1]);
                        if page_count * u64::from(animation.frames) * (texels * 8) as u64
                            > 64 * 1024 * 1024
                        {
                            return Err("Animated material pages exceed the 64 MB bake budget; reduce resolution, frame count or surface coverage".into());
                        }
                        let key = super::pixel_style::bake_key(&job.bundle, job.pixels_per_tile);
                        let cached_pages = page_cache
                            .as_ref()
                            .filter(|(id, k, _)| *id == job.tile && *k == key)
                            .map(|(_, _, pages)| pages.clone());
                        let mut frames = Vec::new();
                        for phase in 0..animation.frames {
                            let time = phase as f32 / animation.frames as f32;
                            let origin = asset
                                .instances
                                .iter()
                                .find(|r| r.id == super::composition::SURFACE_DOMAIN)
                                .map_or([0.; 2], |r| [r.uv_transform[4], r.uv_transform[5]]);
                            let tile_size = super::pixel_style::bake_size(job.pixels_per_tile);
                            let single = render_styled_domain_time(
                                runtime,
                                instance,
                                &device,
                                &queue,
                                tile_size,
                                [1.; 2],
                                origin,
                                job.pixels_per_tile,
                                time,
                            )?;
                            let tile = bake(&single, tile_size);
                            let size = 384;
                            let span = if job.continuous {
                                [2. * job.preview_aspect.max(0.1), 2.]
                            } else {
                                [1., 1.]
                            };
                            let maps = render_styled_domain_time(
                                runtime,
                                instance,
                                &device,
                                &queue,
                                size,
                                span,
                                origin,
                                job.pixels_per_tile,
                                time,
                            )?;
                            let mut warnings: Vec<_> = instance
                                .diagnostics()
                                .into_iter()
                                .map(|d| d.message)
                                .collect();
                            let selection = if let Some(target) = &job.selection {
                                let selection_asset = match target {
                                    PreviewSelection::Operation(id) => super::operation_preview_asset(&asset, id),
                                    PreviewSelection::Source(id) => super::source_preview_asset(id),
                                    PreviewSelection::Input(target, reference) => {
                                        super::input_preview_asset(&asset, target, reference)
                                    }
                                    PreviewSelection::Material(id) => {
                                        let mut selected = asset.clone();
                                        selected.selected_material = id.clone();
                                        selected
                                            .validate()
                                            .map(|_| selected)
                                            .map_err(|e| e.to_string())
                                    }
                                };
                                match selection_asset.and_then(|selection| {
                                    if !selected_compiled
                                        .as_mut()
                                        .is_some_and(|instance| instance.update_asset(&selection).is_ok())
                                    {
                                        selected_compiled = Some(
                                            pollster::block_on(
                                                runtime.compile(&selection, InstanceRecord::new("eldiron-selection")),
                                            )
                                            .map_err(|e| e.to_string())?,
                                        );
                                    }
                                    let maps = render_styled_domain_time(
                                        runtime,
                                        selected_compiled.as_ref().unwrap(),
                                        &device,
                                        &queue,
                                        size,
                                        span,
                                        origin,
                                        job.pixels_per_tile,
                                        time,
                                    )?;
                                    Ok(preview_images(&maps)[0].clone())
                                }) {
                                    Ok(image) => Some((target.clone(), image)),
                                    Err(error) => {
                                        warnings.push(format!("Tile preview: {error}"));
                                        None
                                    }
                                }
                            } else {
                                None
                            };
                            let thumbnails = if job.revision == 0 || phase > 0 {
                                Vec::new()
                            } else {
                                thumbnail_cache.images(runtime, &asset, &job.bundle, &device, &queue, &mut warnings)
                            };
                            let surface_pages = if let Some(layout) = layout.clone().filter(|_| cached_pages.is_none())
                            {
                                if let Some(period) = repeat {
                                    let dimensions = repeat_size.unwrap();
                                    let maps = render_styled_rect_time(runtime, instance, &device, &queue, dimensions, period, [0.;2], job.pixels_per_tile, time)?;
                                    Some(std::sync::Arc::new(SurfacePages { layout, textures: vec![bake_rect(&maps, dimensions)] }))
                                } else {
                                let mut textures: Vec<Option<rusterix::Texture>> =
                                    (0..layout.size[0] * layout.size[1]).map(|_| None).collect();
                                for y in (0..layout.size[1]).step_by(8) {
                                    for x in (0..layout.size[0]).step_by(8) {
                                        let cols = (layout.size[0] - x).min(8);
                                        let rows = (layout.size[1] - y).min(8);
                                        let side = cols.max(rows) as usize;
                                        let units = layout.units_per_page as f32;
                                        let origin = [
                                            (layout.origin[0] as f32 + x as f32) * units,
                                            (layout.origin[1] as f32 + y as f32) * units,
                                        ];
                                        let maps = render_styled_domain_time(
                                            runtime,
                                            instance,
                                            &device,
                                            &queue,
                                            side * tile_size,
                                            [side as f32 * units; 2],
                                            origin,
                                            job.pixels_per_tile,
                                            time,
                                        )?;
                                        for py in 0..rows as usize {
                                            for px in 0..cols as usize {
                                                let page: Maps = std::array::from_fn(|channel| {
                                                    let mut out = Vec::with_capacity(tile_size * tile_size);
                                                    for row in 0..tile_size {
                                                        let at =
                                                            (py * tile_size + row) * side * tile_size + px * tile_size;
                                                        out.extend_from_slice(&maps[channel][at..at + tile_size]);
                                                    }
                                                    out
                                                });
                                                let at = (y as usize + py) * layout.size[0] as usize + x as usize + px;
                                                textures[at] = Some(bake(&page, tile_size));
                                            }
                                        }
                                        let mut superseded = false;
                                        while let Ok(next) = rx.try_recv() {
                                            superseded |= next.tile == job.tile
                                                && (next.bundle != job.bundle
                                                    || next.pixels_per_tile != job.pixels_per_tile);
                                            pending.retain(|old| old.tile != next.tile);
                                            pending.push_back(next);
                                        }
                                        if superseded {
                                            return Err("Superseded material preview".into());
                                        }
                                    }
                                }
                                Some(std::sync::Arc::new(SurfacePages {
                                    layout,
                                    textures: textures.into_iter().map(Option::unwrap).collect(),
                                }))
                                }
                            } else {
                                None
                            };
                            let frame = Frame {
                                animation_images: Vec::new(),
                                animation_selections: Vec::new(),
                                size,
                                images: preview_images(&maps),
                                tile,
                                warnings,
                                selection,
                                continuous: job.continuous,
                                thumbnails,
                                surface_pages,
                            };
                            frames.push(frame);
                        }
                        let images = if animation.frames > 1 {
                            frames.iter().map(|f| f.images.clone()).collect()
                        } else {
                            Vec::new()
                        };
                        let selections = if animation.frames > 1 {
                            frames
                                .iter()
                                .filter_map(|f| f.selection.as_ref().map(|(_, image)| image.clone()))
                                .collect()
                        } else {
                            Vec::new()
                        };
                        let pages = if let Some(mut layout) = layout {
                            if let Some(pages) = cached_pages {
                                Some(pages)
                            } else {
                                layout.animation_frames = animation.frames;
                                let textures = frames
                                    .iter_mut()
                                    .filter_map(|f| f.surface_pages.take())
                                    .flat_map(|pages| std::sync::Arc::try_unwrap(pages).ok().unwrap().textures)
                                    .collect();
                                let pages = std::sync::Arc::new(SurfacePages { layout, textures });
                                page_cache = Some((job.tile, key, pages.clone()));
                                Some(pages)
                            }
                        } else {
                            None
                        };
                        let mut first = frames.remove(0);
                        first.animation_images = images;
                        first.animation_selections = selections;
                        first.surface_pages = pages;
                        Ok(first)
                    });
                    if tx
                        .send(Completed {
                            pixels_per_tile: job.pixels_per_tile,
                            tile: job.tile,
                            bundle: job.bundle,
                            revision: job.revision,
                            frame,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .expect("Material preview worker");
        Self { jobs, results }
    }
    pub fn send(&self, job: Job) {
        let _ = self.jobs.send(job);
    }
}

fn animation_settings(
    asset: &EditableMaterialAsset,
) -> Result<denrim_materials::format::Animation, String> {
    let document = asset
        .resolved(&InstanceRecord::new("animation-metadata"))
        .map_err(|e| e.to_string())?;
    let mut result = denrim_materials::format::Animation::default();
    let mut pending = vec![asset.selected_material.clone()];
    let mut visited = HashSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let material = document
            .materials
            .iter()
            .find(|m| m.id == id)
            .ok_or("Animation source is missing")?;
        let animation = material.animation.validate().map_err(|e| e.to_string())?;
        if animation.frames > 1 {
            if result.frames > 1 && result.frames != animation.frames {
                return Err("Animated composition sources must share frame count".into());
            }
            result = animation;
        }
        if let Some(mix) = &material.mixing {
            pending.extend(mix.inputs.iter().flatten().cloned());
            pending.extend(mix.mask_material.iter().cloned());
        }
    }
    Ok(result)
}

type Maps = [Vec<[f32; 4]>; 5];
#[cfg(test)]
type PreviewMaps = [Vec<[f32; 4]>; 7];
fn render(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: usize,
    span: f32,
) -> Result<Maps, String> {
    render_domain(runtime, instance, device, queue, size, [span, span])
}
fn render_domain(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: usize,
    span: [f32; 2],
) -> Result<Maps, String> {
    render_domain_at(runtime, instance, device, queue, size, span, [0.; 2])
}
fn render_domain_at(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: usize,
    span: [f32; 2],
    origin: [f32; 2],
) -> Result<Maps, String> {
    render_grid(runtime, instance, device, queue, [size; 2], span, origin)
}
fn render_grid(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: [usize; 2],
    span: [f32; 2],
    origin: [f32; 2],
) -> Result<Maps, String> {
    render_grid_time(runtime, instance, device, queue, size, span, origin, 0.)
}
#[cfg(test)]
fn render_styled_domain_at(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: usize,
    span: [f32; 2],
    origin: [f32; 2],
    pixels: u32,
) -> Result<Maps, String> {
    render_styled_domain_time(
        runtime, instance, device, queue, size, span, origin, pixels, 0.,
    )
}
fn render_grid_time(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: [usize; 2],
    span: [f32; 2],
    origin: [f32; 2],
    time: f32,
) -> Result<Maps, String> {
    let request = RenderRequest {
        time,
        resolution: size.map(|v| v as u32),
        domain: SurfaceDomain::rect(origin, span),
    };
    let surface = runtime
        .create_surface(&request)
        .map_err(|e| e.to_string())?;
    let mut encoder = device.create_command_encoder(&Default::default());
    let ticket = runtime
        .encode(&mut encoder, instance, &request, &surface)
        .map_err(|e| e.to_string())?;
    readback_maps(device, queue, encoder, ticket, &surface.maps, size)
}
/// Evaluate on a shared surface grid, then enlarge every channel with nearest
/// sampling. The logical footprint reaches MaterialZ before masks/filtering run.
fn render_styled_domain_time(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: usize,
    span: [f32; 2],
    origin: [f32; 2],
    pixels: u32,
    time: f32,
) -> Result<Maps, String> {
    render_styled_rect_time(
        runtime, instance, device, queue, [size; 2], span, origin, pixels, time,
    )
}
fn render_styled_rect_time(
    runtime: &MaterialZ,
    instance: &MaterialInstance,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: [usize; 2],
    span: [f32; 2],
    origin: [f32; 2],
    pixels: u32,
    time: f32,
) -> Result<Maps, String> {
    if pixels == 0 {
        return render_grid_time(runtime, instance, device, queue, size, span, origin, time);
    }
    let density = (pixels as f32).min((size[0] as f32 / span[0]).min(size[1] as f32 / span[1]));
    let first = origin.map(|v| (v * density).floor());
    let dimensions: [usize; 2] = std::array::from_fn(|i| {
        (((origin[i] + span[i]) * density).ceil() - first[i]).max(1.) as usize
    });
    let snapped = first.map(|v| v / density);
    let extent = dimensions.map(|n| n as f32 / density);
    let maps = render_grid_time(
        runtime, instance, device, queue, dimensions, extent, snapped, time,
    )?;
    Ok(std::array::from_fn(|channel| {
        let mut output = Vec::with_capacity(size[0] * size[1]);
        for y in 0..size[1] {
            for x in 0..size[0] {
                let cell: [usize; 2] = std::array::from_fn(|i| {
                    let p = [x, y][i] as f32 + 0.5;
                    (((origin[i] + p / size[i] as f32 * span[i]) * density).floor() - first[i])
                        .max(0.) as usize
                });
                output.push(
                    maps[channel][cell[1].min(dimensions[1] - 1) * dimensions[0]
                        + cell[0].min(dimensions[0] - 1)],
                );
            }
        }
        output
    }))
}

fn readback_maps<const N: usize>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    ticket: RenderTicket,
    maps: &[SurfaceMap; N],
    size: [usize; 2],
) -> Result<[Vec<[f32; 4]>; N], String> {
    let buffers: Vec<_> = maps
        .iter()
        .map(|map| {
            let pixel = if map.layout.format == wgpu::TextureFormat::Rgba16Float {
                8
            } else {
                4
            };
            let stride = (size[0] as u32 * pixel).div_ceil(256) * 256;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Eldiron material readback"),
                size: u64::from(stride) * size[1] as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                map.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(size[1] as u32),
                    },
                },
                map.texture.size(),
            );
            (buffer, stride as usize, pixel as usize)
        })
        .collect();
    queue.submit([encoder.finish()]);
    let (tx, rx) = mpsc::channel();
    ticket.map_diagnostics(move |r| {
        let _ = tx.send(r);
    });
    let callbacks: Vec<_> = buffers
        .iter()
        .map(|(buffer, _, _)| {
            let (tx, rx) = mpsc::channel();
            buffer.map_async(wgpu::MapMode::Read, .., move |r| {
                let _ = tx.send(r);
            });
            rx
        })
        .collect();
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(1))
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    for callback in callbacks {
        callback
            .recv_timeout(Duration::from_secs(1))
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
    }
    let mut result: [Vec<[f32; 4]>; N] =
        std::array::from_fn(|_| Vec::with_capacity(size[0] * size[1]));
    for (index, (buffer, stride, pixel)) in buffers.into_iter().enumerate() {
        let data = buffer.get_mapped_range(..);
        for y in 0..size[1] {
            for x in 0..size[0] {
                let at = y * stride + x * pixel;
                let value = if pixel == 8 {
                    std::array::from_fn(|c| {
                        half::f16::from_bits(u16::from_le_bytes(
                            data[at + c * 2..at + c * 2 + 2].try_into().unwrap(),
                        ))
                        .to_f32()
                    })
                } else {
                    [
                        f32::from_le_bytes(data[at..at + 4].try_into().unwrap()),
                        0.,
                        0.,
                        1.,
                    ]
                };
                if value.iter().any(|v| !v.is_finite()) {
                    return Err("Material returned a non-finite output".into());
                }
                result[index].push(value);
            }
        }
        drop(data);
        buffer.unmap();
    }
    Ok(result)
}
fn srgb(v: f32) -> u8 {
    let v = v.max(0.);
    let encoded = if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    };
    (encoded.clamp(0., 1.) * 255.).round() as u8
}
fn unit(v: f32) -> u8 {
    (v.clamp(0., 1.) * 255.).round() as u8
}
fn preview_images<const N: usize>(maps: &[Vec<[f32; 4]>; N]) -> [Vec<u8>; N] {
    std::array::from_fn(|channel| {
        maps[channel]
            .iter()
            .enumerate()
            .flat_map(|(i, value)| {
                if channel == 0 {
                    [
                        srgb(value[0]),
                        srgb(value[1]),
                        srgb(value[2]),
                        unit(maps[1][i][0]),
                    ]
                } else if channel == 4 {
                    [srgb(value[0]), srgb(value[1]), srgb(value[2]), 255]
                } else if channel == 6 {
                    [unit(value[0]), unit(value[1]), unit(value[2]), 255]
                } else {
                    let v = unit(value[0]);
                    [v, v, v, 255]
                }
            })
            .collect()
    })
}
/// Eldiron stores R/M/O/E as four 4-bit scalars. Full precision MZ outputs
/// remain available in the editor; emission hue/HDR cannot fit this tile cache.
/// Opacity belongs to RGBA alpha exactly once; material opacity stays 1 because
/// SceneVM multiplies the two during shading.
fn bake(maps: &Maps, size: usize) -> rusterix::Texture {
    bake_rect(maps, [size; 2])
}
fn bake_rect(maps: &Maps, size: [usize; 2]) -> rusterix::Texture {
    let rgba = preview_images(maps)[0].clone();
    let mut texture = rusterix::Texture::alloc(size[0], size[1]);
    texture.data = rgba;
    for y in 0..size[1] {
        for x in 0..size[0] {
            let i = y * size[0] + x;
            texture.set_materials(
                x as u32,
                y as u32,
                maps[2][i][0],
                maps[3][i][0],
                1.,
                maps[4][i][0].max(maps[4][i][1]).max(maps[4][i][2]),
            );
        }
    }
    texture
}

#[cfg(test)]
mod tests {
    use super::*;
    const THUMBNAIL_FIXTURE: &str = r#"project 1
material "surface" called "Initial thumbnails"
  mode pbr
  tiling raw
  row "color"
    use "Perlin Noise" id "first-noise"
    use "Mix" id "pigment"
      set "color1" to 0.08 0.16 0.025 1
      set "color2" to 0.4 0.6 0.15 1
      set "factor" to 1 1 1
  row "roughness"
    use "Perlin Noise" id "roughness-noise"
  row "unconnected"
    use "Color" id "unconnected-color"
      set "color" to 1 0 0 1
  output base color from row "color" xyz
  output roughness from row "roughness" x
"#;
    #[test]
    #[ignore = "renders a palette/material contact sheet on a native GPU"]
    fn embedded_starter_palette_contact_sheet() {
        use crate::prelude::*;
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let mut project = Project::default();
        super::super::starters::install(&mut project).unwrap();
        let worker = Worker::new(device, queue);
        let side = 192;
        let mut pixels = vec![0; side * 3 * side * 2 * 4];
        for (index, name) in ["Grass", "Rock", "Hills", "Street", "Walls", "Water"]
            .into_iter()
            .enumerate()
        {
            let id = *project
                .tiles
                .iter()
                .find(|(_, tile)| tile.alias == name)
                .unwrap()
                .0;
            worker.send(Job {
                pixels_per_tile: 16,
                tile: id,
                revision: 0,
                bundle: project.procedural_materials[&id].clone(),
                continuous: true,
                selection: None,
                preview_aspect: 1.,
            });
            let frame = worker
                .results
                .recv_timeout(Duration::from_secs(60))
                .unwrap()
                .frame
                .unwrap();
            assert!(frame.warnings.is_empty(), "{name}: {:?}", frame.warnings);
            for y in 0..side {
                for x in 0..side {
                    let src = ((y * frame.size / side) * frame.size + x * frame.size / side) * 4;
                    let dst = (((index / 3 * side + y) * side * 3) + (index % 3 * side + x)) * 4;
                    pixels[dst..dst + 4].copy_from_slice(&frame.images[0][src..src + 4]);
                }
            }
        }
        let path = std::env::var("ELDIRON_MATERIAL_PREVIEW")
            .unwrap_or_else(|_| "/tmp/eldiron-material-palette-review.png".into());
        let mut encoder = png::Encoder::new(
            std::fs::File::create(path).unwrap(),
            (side * 3) as u32,
            (side * 2) as u32,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    #[test]
    fn straight_color_and_opacity_are_preserved_in_preview_and_bake() {
        let maps: Maps = [
            vec![[0.25, 0.5, 1., 1.]],
            vec![[0.25, 0., 0., 1.]],
            vec![[0.7, 0., 0., 1.]],
            vec![[0.2, 0., 0., 1.]],
            vec![[0.1, 0.2, 0.3, 1.]],
        ];
        let images = preview_images(&maps);
        assert_eq!(images[0], vec![137, 188, 255, 64]);
        assert_eq!(images[1], vec![64, 64, 64, 255]);
        let tile = bake(&maps, 1);
        assert_eq!(tile.data, images[0]);
        assert_eq!(
            tile.get_materials(0, 0).2,
            1.,
            "opacity must not be multiplied twice"
        );
        assert!(!tile.material_bytes().is_empty());
    }
    #[test]
    fn height_and_normal_previews_preserve_linear_data_channels() {
        let maps: PreviewMaps = [
            vec![[0.25, 0.5, 1., 1.]],
            vec![[1., 0., 0., 1.]],
            vec![[0.7, 0., 0., 1.]],
            vec![[0.2, 0., 0., 1.]],
            vec![[0.1, 0.2, 0.3, 1.]],
            vec![[0.4, 0., 0., 1.]],
            vec![[0.25, 0.5, 1., 1.]],
        ];
        let images = preview_images(&maps);
        assert_eq!(images[5], vec![102, 102, 102, 255]);
        assert_eq!(images[6], vec![64, 128, 255, 255]);
    }
    #[test]
    #[ignore = "requires a native GPU adapter; writes animated 2D/3D previews"]
    fn repeat_water_composition_animates_in_2d_and_3d_with_stationary_grass() {
        use crate::prelude::*;
        use scenevm::{
            Atom, Camera3D, CameraKind, Chunk, GeoId, Poly2D, Poly3D, RenderMode, SceneVM,
        };
        use shared::material_composition::{CompositionLayer, MaterialComposition};
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let mut project = Project::default();
        super::super::starters::install(&mut project).unwrap();
        let grass = project
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Grass")
            .map(|(id, _)| *id)
            .unwrap();
        let water = project
            .tiles
            .iter()
            .find(|(_, t)| t.alias == "Water")
            .map(|(id, _)| *id)
            .unwrap();
        let id = Uuid::new_v4();
        let composition = MaterialComposition {
            base: grass,
            layers: vec![CompositionLayer::split(water)],
            tiling: shared::material_composition::CompositionTiling {
                mode: "repeat".into(),
                ..Default::default()
            },
        };
        let bundle = super::super::composition::bundle(id, &project, Some(&composition)).unwrap();
        let worker = Worker::new(device, queue);
        worker.send(Job {
            pixels_per_tile: 16,
            tile: id,
            revision: 1,
            bundle: bundle.clone(),
            continuous: true,
            selection: None,
            preview_aspect: 1.,
        });
        let frame = worker
            .results
            .recv_timeout(Duration::from_secs(120))
            .unwrap()
            .frame
            .unwrap();
        assert_eq!(frame.animation_images.len(), 16);
        let pages = frame.surface_pages.as_ref().unwrap();
        assert_eq!(pages.layout.animation_frames, 16);
        assert_eq!(pages.layout.repeat_period, [1., 1.]);
        assert_eq!(pages.layout.size, [1, 1]);
        assert_eq!(pages.textures.len(), 16);
        let mut changed = [false; 2];
        for y in 0..16 {
            for x in 0..16 {
                let i = (y * 16 + x) * 4;
                if frame.tile.data[i..i + 4] != pages.textures[8].data[i..i + 4] {
                    changed[usize::from(x >= 8)] = true;
                }
            }
        }
        assert_eq!(
            changed,
            [false, true],
            "Grass stays still while the water moves"
        );
        let mut runtime_tile = rusterix::Tile::from_textures(pages.textures.clone());
        runtime_tile.uv_pages = Some(pages.layout.clone());
        runtime_tile.baked_material_data =
            pages.textures.iter().map(|t| t.material_bytes()).collect();
        let saved = serde_json::to_string(&runtime_tile).unwrap();
        let restored: rusterix::Tile = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.uv_pages.as_ref().unwrap().animation_frames, 16);
        assert_eq!(restored.textures.len(), 16);
        for (name, mode) in [("2d", RenderMode::Raster2D), ("3d", RenderMode::Raster3D)] {
            let mut scene = SceneVM::new(128, 128);
            scene.execute(Atom::SetRenderMode(mode));
            scene.execute(Atom::SetBackground(Vec4::new(0.025, 0.025, 0.025, 1.)));
            scene.execute(Atom::SetGP3(Vec4::one()));
            scene.execute(Atom::SetGP8(Vec4::new(0., 0., 1., 1.)));
            scene.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 2.2)));
            scene.execute(Atom::AddTile {
                id,
                width: 16,
                height: 16,
                frames: pages.textures.iter().map(|t| t.data.clone()).collect(),
                material_frames: Some(
                    pages
                        .textures
                        .iter()
                        .map(|t| t.data_ext.clone().unwrap())
                        .collect(),
                ),
            });
            scene.execute(Atom::SetTileUvPages {
                id,
                origin: [0, 0],
                size: [1, 1],
                axes: [[0.; 3]; 2],
                units_per_page: 1,
                animation_frames: 16,
                repeat_period: pages.layout.repeat_period,
            });
            scene.execute(Atom::BuildAtlas);
            let mut chunk = Chunk::new(Vec2::zero(), 16);
            if name == "2d" {
                chunk.add(Poly2D::poly(
                    GeoId::Unknown(1),
                    id,
                    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
                    vec![[0.; 2]; 4],
                    vec![(0, 1, 2), (0, 2, 3)],
                ));
                scene.execute(Atom::SetTransform2D(Mat3::scaling_3d(Vec3::new(
                    128., 128., 1.,
                ))));
            } else {
                chunk.add_3d(Poly3D::poly(
                    GeoId::Unknown(1),
                    id,
                    vec![
                        [0., 0., 0., 1.],
                        [1., 0., 0., 1.],
                        [1., 0., 1., 1.],
                        [0., 0., 1., 1.],
                    ],
                    vec![[0.; 2]; 4],
                    vec![(0, 2, 1), (0, 3, 2)],
                ));
                let camera = Camera3D::builder()
                    .with_kind(CameraKind::OrthoIso)
                    .with_pos(Vec3::new(0.5, 2., 2.))
                    .with_basis(Vec3::new(0., -2., -1.5), Vec3::unit_y())
                    .with_ortho_half_h(0.7);
                scene.execute(Atom::SetCamera3D { camera });
            }
            scene.execute(Atom::AddChunk {
                id: Uuid::new_v4(),
                chunk,
            });
            let mut filmstrip = vec![0; 128 * 128 * 16 * 4];
            let mut first = Vec::new();
            let mut middle = Vec::new();
            let mut pixels = vec![0; 128 * 128 * 4];
            for phase in 0..16 {
                scene.execute(Atom::SetAnimationCounter(phase as usize));
                scene.render_frame(&mut pixels, 128, 128);
                if phase == 0 {
                    first = pixels.clone();
                }
                if phase == 8 {
                    middle = pixels.clone();
                }
                for y in 0..128 {
                    let dst = (y * 128 * 16 + phase as usize * 128) * 4;
                    filmstrip[dst..dst + 128 * 4]
                        .copy_from_slice(&pixels[y * 128 * 4..(y + 1) * 128 * 4]);
                }
            }
            assert_ne!(first, middle, "Water must move in {name}");
            scene.execute(Atom::SetAnimationCounter(16));
            scene.render_frame(&mut pixels, 128, 128);
            assert_eq!(pixels, first, "Loop closes in {name}");
            let output =
                std::fs::File::create(format!("/tmp/eldiron-water-{name}-frames.png")).unwrap();
            let mut encoder = png::Encoder::new(output, 128 * 16, 128);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&filmstrip)
                .unwrap();
        }
    }
    #[test]
    #[ignore = "requires a GPU device"]
    fn saved_tiling_switches_between_world_pages_and_one_shared_repeat_pattern() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = Worker::new(device, queue);
        let id = theframework::prelude::Uuid::new_v4();
        let mut asset =
            EditableMaterialAsset::parse(&id.to_string(), THUMBNAIL_FIXTURE, "surface").unwrap();
        let layout = rusterix::TileUvPages {
            animation_frames: 1,
            repeat_period: [0.; 2],
            origin: [-1, -1],
            size: [2, 1],
            axes: [[1., 0., 0.], [0., 0., 1.]],
            units_per_page: 1,
        };
        super::super::composition::surface_pages::apply(&mut asset, Some(&layout));
        for (mode, period) in [
            ("raw", [1., 1.]),
            ("repeat", [2., 1.]),
            ("seamless", [1., 2.]),
            ("raw", [1., 1.]),
        ] {
            asset.root.project.materials[0].tiling.period = period;
            asset.root.project.materials[0].tiling.mode =
                denrim_materials::periodic::Mode::parse(mode).unwrap();
            worker.send(Job {
                pixels_per_tile: 16,
                tile: id,
                revision: 0,
                bundle: asset.save().unwrap(),
                continuous: true,
                selection: None,
                preview_aspect: 1.,
            });
            let frame = worker
                .results
                .recv_timeout(Duration::from_secs(60))
                .unwrap()
                .frame
                .unwrap();
            assert!(frame.warnings.is_empty(), "{:?}", frame.warnings);
            let pages = frame.surface_pages.unwrap();
            if mode == "raw" {
                assert_ne!(pages.textures[0].data, pages.textures[1].data);
            } else {
                assert_eq!(
                    pages.textures.len(),
                    1,
                    "{mode} must bake one pattern, independent of world coverage"
                );
                assert_eq!(pages.layout.size, [1, 1]);
                assert_eq!(pages.layout.repeat_period, period);
            }
            assert_eq!(pages.textures[0].width, (period[0] * 16.) as usize);
            assert_eq!(pages.textures[0].height, (period[1] * 16.) as usize);
        }
    }

    #[test]
    #[ignore = "requires a GPU device"]
    fn global_resolution_sizes_tiles_and_world_pages_without_upscaling() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = Worker::new(device, queue);
        let id = theframework::prelude::Uuid::new_v4();
        let mut asset =
            EditableMaterialAsset::parse(&id.to_string(), THUMBNAIL_FIXTURE, "surface").unwrap();
        let layout = rusterix::TileUvPages {
            animation_frames: 1,
            repeat_period: [0.; 2],
            origin: [0, 0],
            size: [2, 1],
            axes: [[1., 0., 0.], [0., 0., 1.]],
            units_per_page: 1,
        };
        super::super::composition::surface_pages::apply(&mut asset, Some(&layout));
        let bundle = asset.save().unwrap();
        // Reusing one worker also checks that changing only resolution refreshes
        // the cached world pages, including a non-power-of-two grid.
        for (pixels, size) in [(16, 16), (32, 32), (24, 24), (0, 64), (16, 16)] {
            worker.send(Job {
                pixels_per_tile: pixels,
                tile: id,
                revision: 0,
                bundle: bundle.clone(),
                continuous: false,
                selection: None,
                preview_aspect: 1.,
            });
            let completed = worker
                .results
                .recv_timeout(Duration::from_secs(60))
                .unwrap();
            assert_eq!(completed.pixels_per_tile, pixels);
            let frame = completed.frame.unwrap();
            assert!(frame.warnings.is_empty(), "{:?}", frame.warnings);
            assert_eq!((frame.tile.width, frame.tile.height), (size, size));
            assert_eq!(frame.tile.data.len(), size * size * 4);
            let pages = frame.surface_pages.unwrap();
            assert_eq!(pages.textures.len(), 2);
            for page in &pages.textures {
                assert_eq!((page.width, page.height), (size, size));
                assert_eq!(page.data.len(), size * size * 4);
                assert_eq!(
                    page.material_bytes().len(),
                    frame.tile.material_bytes().len()
                );
            }
            assert_eq!(pages.textures[0].data, frame.tile.data);
            assert_eq!(
                pages.textures[0].material_bytes(),
                frame.tile.material_bytes()
            );
            assert_ne!(
                pages.textures[0].data, pages.textures[1].data,
                "World pages should retain continuous variation"
            );
            assert_eq!(frame.size, 384);
            assert!(frame.images.iter().all(|map| map.len() == 384 * 384 * 4));
        }
    }

    #[test]
    #[ignore = "requires a GPU device"]
    fn pixel_grid_is_shared_across_pages_and_all_pbr_channels() {
        pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
            let runtime = MaterialZ::new(&device, &queue).await.unwrap();
            let asset =
                EditableMaterialAsset::parse("pixel-grid", THUMBNAIL_FIXTURE, "surface").unwrap();
            let instance = runtime
                .compile(&asset, InstanceRecord::new("grid"))
                .await
                .unwrap();
            let generation = instance.pipeline_generation();
            let left = render_styled_domain_at(
                &runtime,
                &instance,
                &device,
                &queue,
                64,
                [1., 1.],
                [-1., 0.],
                16,
            )
            .unwrap();
            let wide = render_styled_domain_at(
                &runtime,
                &instance,
                &device,
                &queue,
                128,
                [2., 1.],
                [-1., 0.],
                16,
            )
            .unwrap();
            for channel in 0..5 {
                for y in 0..64 {
                    for x in 0..64 {
                        assert_eq!(
                            left[channel][y * 64 + x],
                            left[channel][(y / 4 * 4) * 64 + x / 4 * 4],
                            "Every PBR map must share 4x4 logical pixels"
                        );
                        assert_eq!(
                            left[channel][y * 64 + x],
                            wide[channel][y * 2 * 128 + x],
                            "Page boundaries and aspect must not change the grid"
                        );
                    }
                }
            }
            let clipped = render_styled_domain_at(
                &runtime,
                &instance,
                &device,
                &queue,
                64,
                [1., 0.5],
                [-0.03, 0.02],
                16,
            )
            .unwrap();
            let reference = render_grid(
                &runtime,
                &instance,
                &device,
                &queue,
                [18, 9],
                [18. / 16., 9. / 16.],
                [-1. / 16., 0.],
            )
            .unwrap();
            for channel in 0..5 {
                for y in 0..64 {
                    for x in 0..64 {
                        let cx = (((-0.03 + (x as f32 + 0.5) / 64.) * 16.).floor() + 1.) as usize;
                        let cy = (((0.02 + (y as f32 + 0.5) / 64. * 0.5) * 16.).floor()) as usize;
                        let actual = clipped[channel][y * 64 + x];
                        let expected = reference[channel][cy * 18 + cx];
                        assert!(
                            actual
                                .iter()
                                .zip(expected)
                                .all(|(a, b)| (a - b).abs() < 0.0001)
                        );
                    }
                }
            }
            assert_eq!(instance.pipeline_generation(), generation);
        });
    }

    #[test]
    #[ignore = "requires a GPU device"]
    fn starter_renders_and_constant_edits_reuse_the_pipeline() {
        pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
            let runtime = MaterialZ::new(&device, &queue).await.unwrap();
            let asset = super::super::starter(theframework::prelude::Uuid::new_v4()).unwrap();
            let mut instance = runtime
                .compile(&asset, InstanceRecord::new("test"))
                .await
                .unwrap();
            let generation = instance.pipeline_generation();
            let maps = render(&runtime, &instance, &device, &queue, 8, 1.).unwrap();
            let original = maps[0][0][0];
            let continuous = render(&runtime, &instance, &device, &queue, 24, 3.).unwrap();
            for y in 0..8 {
                for x in 0..8 {
                    for channel in 0..5 {
                        for component in 0..4 {
                            assert!(
                                (maps[channel][y * 8 + x][component]
                                    - continuous[channel][y * 24 + x][component])
                                    .abs()
                                    < 0.002
                            );
                        }
                    }
                }
            }
            let next = super::super::edited(
                &asset,
                crate::material_editor::MaterialEditorCommand::SetNumber {
                    operation: "pigment".into(),
                    parameter: "color1:0".into(),
                    value: 0.8,
                },
            )
            .unwrap();
            instance.update_asset(&next).unwrap();
            assert_eq!(generation, instance.pipeline_generation());
            let maps = render(&runtime, &instance, &device, &queue, 8, 1.).unwrap();
            assert!((maps[0][0][0] - original).abs() > 0.05);
        });
    }
    #[test]
    #[ignore = "requires a GPU device"]
    fn initial_worker_frame_contains_all_operation_thumbnails_without_selection() {
        let (device, queue) = pollster::block_on(async {
            let adapter = wgpu::Instance::default()
                .request_adapter(&Default::default())
                .await
                .unwrap();
            adapter.request_device(&Default::default()).await.unwrap()
        });
        let worker = Worker::new(device, queue);
        let id = theframework::prelude::Uuid::new_v4();
        let asset =
            EditableMaterialAsset::parse(&id.to_string(), THUMBNAIL_FIXTURE, "surface").unwrap();
        let bundle = asset.save().unwrap();
        worker.send(Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 1,
            bundle: bundle.clone(),
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let initial = worker
            .results
            .recv_timeout(Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        assert!(initial.selection.is_none());
        assert!(initial.warnings.is_empty(), "{:?}", initial.warnings);
        assert_eq!(initial.thumbnails.len(), 4);
        let thumbnails: HashMap<_, _> = initial
            .thumbnails
            .iter()
            .map(|(id, image)| (id.as_str(), image.as_slice()))
            .collect();
        for id in [
            "first-noise",
            "pigment",
            "roughness-noise",
            "unconnected-color",
        ] {
            assert_eq!(thumbnails[id].len(), THUMBNAIL_SIZE * THUMBNAIL_SIZE * 4);
        }
        let noise = thumbnails["first-noise"];
        assert!(
            noise
                .chunks_exact(4)
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2] && pixel[3] == 255)
        );
        let levels: HashSet<_> = noise.chunks_exact(4).map(|pixel| pixel[0]).collect();
        assert!(
            levels.len() > 8,
            "The initial Perlin thumbnail must show its pattern"
        );
        assert!(
            thumbnails["pigment"]
                .chunks_exact(4)
                .any(|pixel| pixel[0] != pixel[1])
        );
        assert!(
            thumbnails["unconnected-color"]
                .chunks_exact(4)
                .all(|pixel| pixel == [255, 0, 0, 255])
        );
        worker.send(Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 2,
            bundle: bundle.clone(),
            continuous: true,
            selection: None,
            preview_aspect: 2.5,
        });
        let repeated = worker
            .results
            .recv_timeout(Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        assert_eq!(
            repeated.thumbnails, initial.thumbnails,
            "Minimap domain changes must not regenerate different operation previews"
        );
        worker.send(Job {
            pixels_per_tile: 0,
            tile: id,
            revision: 0,
            bundle,
            continuous: false,
            selection: None,
            preview_aspect: 1.,
        });
        let background = worker
            .results
            .recv_timeout(Duration::from_secs(60))
            .unwrap()
            .frame
            .unwrap();
        assert!(
            background.thumbnails.is_empty(),
            "Background bakes do not need graph thumbnails"
        );
    }
}

#[cfg(test)]
#[path = "icon_experiment.rs"]
mod icon_experiment;

#[cfg(test)]
#[path = "image_source_gpu.rs"]
mod image_source_gpu;
