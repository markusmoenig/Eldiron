//! Explicit CPU asset production. Native workers and browser cooperative jobs
//! share the same domains, animation/pages, quantization and readback policy.
//! Interactive previews never enter this module.
use super::{
    bake_pixels::{bake, bake_rect, preview_images},
    bake_types::*,
};
use denrim_materials::{
    gpu::*,
    integration::{EditableMaterialAsset, InstanceRecord},
};
use renderkit::GpuContext;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
type Maps = [Vec<[f32; 4]>; 5];

pub(super) struct Baker {
    context: Arc<GpuContext>,
    canceled: Arc<AtomicBool>,
    runtime: Option<MaterialZ>,
    compiled: Option<MaterialInstance>,
}

impl Baker {
    pub fn new(context: Arc<GpuContext>, canceled: Arc<AtomicBool>) -> Self {
        Self {
            context,
            canceled,
            runtime: None,
            compiled: None,
        }
    }
    fn current(&self) -> Result<(), String> {
        if self.canceled.load(Ordering::Acquire) || self.context.is_lost() {
            Err("Material asset bake was retired".into())
        } else {
            Ok(())
        }
    }
    pub async fn bake(&mut self, job: &Job) -> Result<Frame, String> {
        self.current()?;
        let asset = EditableMaterialAsset::reopen(&job.bundle).map_err(|e| e.to_string())?;
        if self.runtime.is_none() {
            self.runtime = Some(
                MaterialZ::new_with_context(self.context.clone())
                    .await
                    .map_err(|e| e.to_string())?,
            );
        }
        self.current()?;
        let runtime = self.runtime.as_ref().unwrap();
        if !self
            .compiled
            .as_mut()
            .is_some_and(|instance| instance.update_asset(&asset).is_ok())
        {
            self.compiled = Some(
                runtime
                    .compile(&asset, InstanceRecord::new("eldiron-asset-bake"))
                    .await
                    .map_err(|e| e.to_string())?,
            );
        }
        self.current()?;
        let instance = self.compiled.as_ref().unwrap();
        let animation = super::preview_metadata::animation_settings(&asset)?;
        let root = asset
            .root
            .project
            .materials
            .iter()
            .find(|m| m.id == asset.selected_material)
            .ok_or("Missing material")?;
        let repeat = (root.tiling.mode != denrim_materials::periodic::Mode::Raw)
            .then_some(root.tiling.period);
        let mut layout = super::composition::surface_pages::decode(&asset)?;
        if let Some(period) = repeat {
            layout = Some(rusterix::TileUvPages {
                animation_frames: 1,
                repeat_period: period,
                origin: [0; 2],
                size: [1; 2],
                axes: [[0.; 3]; 2],
                units_per_page: 1,
            });
        }
        if animation.frames > 1 && layout.is_none() {
            layout = Some(rusterix::TileUvPages {
                animation_frames: 1,
                repeat_period: [0.; 2],
                origin: [0; 2],
                size: [1; 2],
                axes: [[0.; 3]; 2],
                units_per_page: 1,
            });
        }
        let side = super::pixel_style::bake_size(job.pixels_per_tile);
        let dimensions =
            repeat.map(|period| period.map(|v| (v * side as f32).ceil().max(1.) as usize));
        if dimensions.is_some_and(|size| size.iter().any(|v| *v > 8192)) {
            return Err("Repeating material period exceeds the 8192 pixel texture limit".into());
        }
        let texels = dimensions.map_or(side * side, |s| s[0] * s[1]);
        super::composition::surface_pages::fit_bake_budget(&mut layout, texels, animation.frames)?;
        let origin = asset
            .instances
            .iter()
            .find(|r| r.id == super::composition::SURFACE_DOMAIN)
            .map_or([-0.5; 2], |r| [r.uv_transform[4], r.uv_transform[5]]);
        let mut animation_images = Vec::new();
        let mut first = None;
        let mut pages = Vec::new();
        for phase in 0..animation.frames {
            self.current()?;
            let time = phase as f32 / animation.frames as f32;
            let maps = self
                .render(
                    runtime,
                    instance,
                    [side; 2],
                    [1.; 2],
                    origin,
                    job.pixels_per_tile,
                    time,
                )
                .await?;
            let images = preview_images(&maps);
            if animation.frames > 1 {
                animation_images.push(images.clone());
            }
            if first.is_none() {
                first = Some((bake(&maps, side), images));
            }
            if let Some(layout) = &layout {
                if let Some(period) = repeat {
                    let dimensions = dimensions.unwrap();
                    let maps = self
                        .render(
                            runtime,
                            instance,
                            dimensions,
                            period,
                            period.map(|v| -0.5 * v),
                            job.pixels_per_tile,
                            time,
                        )
                        .await?;
                    pages.push(bake_rect(&maps, dimensions));
                } else {
                    let mut textures: Vec<Option<rusterix::Texture>> =
                        (0..layout.size[0] * layout.size[1]).map(|_| None).collect();
                    for y in (0..layout.size[1]).step_by(8) {
                        for x in (0..layout.size[0]).step_by(8) {
                            self.current()?;
                            let cols = (layout.size[0] - x).min(8);
                            let rows = (layout.size[1] - y).min(8);
                            let count = cols.max(rows) as usize;
                            let units = layout.units_per_page as f32;
                            let origin = [
                                (layout.origin[0] as f32 + x as f32) * units,
                                (layout.origin[1] as f32 + y as f32) * units,
                            ];
                            let maps = self
                                .render(
                                    runtime,
                                    instance,
                                    [count * side; 2],
                                    [count as f32 * units; 2],
                                    origin,
                                    job.pixels_per_tile,
                                    time,
                                )
                                .await?;
                            for py in 0..rows as usize {
                                for px in 0..cols as usize {
                                    let page: Maps = std::array::from_fn(|channel| {
                                        let mut out = Vec::with_capacity(side * side);
                                        for row in 0..side {
                                            let at = (py * side + row) * count * side + px * side;
                                            out.extend_from_slice(&maps[channel][at..at + side]);
                                        }
                                        out
                                    });
                                    textures[(y as usize + py) * layout.size[0] as usize
                                        + x as usize
                                        + px] = Some(bake(&page, side));
                                }
                            }
                        }
                    }
                    pages.extend(textures.into_iter().map(Option::unwrap));
                }
            }
        }
        self.current()?;
        let (tile, images) = first.ok_or("Material animation has no frames")?;
        let surface_pages = layout.map(|mut layout| {
            layout.animation_frames = animation.frames;
            Arc::new(SurfacePages {
                layout,
                textures: pages,
            })
        });
        Ok(Frame {
            size: side,
            animation_images,
            animation_selections: Vec::new(),
            images,
            tile,
            warnings: instance
                .diagnostics()
                .into_iter()
                .map(|d| d.message)
                .collect(),
            selection: None,
            continuous: job.continuous,
            thumbnails: Vec::new(),
            surface_pages,
        })
    }
    async fn render(
        &self,
        runtime: &MaterialZ,
        instance: &MaterialInstance,
        size: [usize; 2],
        span: [f32; 2],
        origin: [f32; 2],
        pixels: u32,
        time: f32,
    ) -> Result<Maps, String> {
        self.current()?;
        let density = (pixels != 0)
            .then(|| (pixels as f32).min((size[0] as f32 / span[0]).min(size[1] as f32 / span[1])));
        let first = density
            .map(|d| origin.map(|v| (v * d).floor()))
            .unwrap_or([0.; 2]);
        let dimensions: [usize; 2] = density
            .map(|d| {
                std::array::from_fn(|i| {
                    (((origin[i] + span[i]) * d).ceil() - first[i]).max(1.) as usize
                })
            })
            .unwrap_or(size);
        let request = RenderRequest {
            time,
            resolution: dimensions.map(|n| n as u32),
            domain: SurfaceDomain::rect(
                density.map(|d| first.map(|v| v / d)).unwrap_or(origin),
                density
                    .map(|d| dimensions.map(|v| v as f32 / d))
                    .unwrap_or(span),
            ),
        };
        let surface = runtime
            .create_surface(&request)
            .map_err(|e| e.to_string())?;
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("MaterialZ explicit asset bake"),
                });
        let ticket = runtime
            .encode(&mut encoder, instance, &request, &surface)
            .map_err(|e| e.to_string())?;
        let maps = self
            .readback(encoder, ticket, &surface.maps, dimensions)
            .await?;
        self.current()?;
        if let Some(density) = density {
            Ok(std::array::from_fn(|channel| {
                let mut out = Vec::with_capacity(size[0] * size[1]);
                for y in 0..size[1] {
                    for x in 0..size[0] {
                        let cell: [usize; 2] = std::array::from_fn(|i| {
                            (((origin[i] + ([x, y][i] as f32 + 0.5) / size[i] as f32 * span[i])
                                * density)
                                .floor()
                                - first[i])
                                .max(0.) as usize
                        });
                        out.push(
                            maps[channel][cell[1].min(dimensions[1] - 1) * dimensions[0]
                                + cell[0].min(dimensions[0] - 1)],
                        );
                    }
                }
                out
            }))
        } else {
            Ok(maps)
        }
    }
    async fn readback(
        &self,
        mut encoder: wgpu::CommandEncoder,
        mut ticket: RenderTicket,
        maps: &[SurfaceMap; 5],
        size: [usize; 2],
    ) -> Result<Maps, String> {
        let buffers: Vec<_> = maps
            .iter()
            .map(|map| {
                let pixel = match map.layout.format {
                    wgpu::TextureFormat::Rgba16Float => 8,
                    wgpu::TextureFormat::R32Float => 4,
                    _ => return Err("Unsupported material bake format".into()),
                };
                let stride = (size[0] * pixel).div_ceil(256) * 256;
                let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("MaterialZ explicit asset readback"),
                    size: (stride * size[1]) as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_texture_to_buffer(
                    map.texture.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(stride as u32),
                            rows_per_image: Some(size[1] as u32),
                        },
                    },
                    map.texture.size(),
                );
                Ok((buffer, stride, pixel))
            })
            .collect::<Result<_, String>>()?;
        self.context.record_asset_bake(
            buffers
                .iter()
                .map(|(_, _, pixel)| (size[0] * size[1] * pixel) as u64)
                .sum(),
        );
        self.context.queue.submit([encoder.finish()]);
        ticket.submitted();
        let (done, ready) = futures::channel::oneshot::channel();
        ticket.map_diagnostics(move |result| {
            let _ = done.send(result);
        });
        let pending: Vec<_> = buffers
            .iter()
            .map(|(buffer, _, _)| {
                let (done, ready) = futures::channel::oneshot::channel();
                buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                    let _ = done.send(result);
                });
                ready
            })
            .collect();
        #[cfg(not(target_arch = "wasm32"))]
        self.context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .map_err(|e| e.to_string())?;
        ready
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        for result in pending {
            result
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
        }
        self.current()?;
        let mut result: Maps = std::array::from_fn(|_| Vec::with_capacity(size[0] * size[1]));
        for (channel, (buffer, stride, pixel)) in buffers.into_iter().enumerate() {
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
                    result[channel].push(value);
                }
            }
            drop(data);
            buffer.unmap();
        }
        Ok(result)
    }
}
