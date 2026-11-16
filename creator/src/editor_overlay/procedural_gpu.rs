//! Explicit-device GPU icon production and bounded retained source residency.
use super::{OverlayImages, procedural::ProceduralIcon};
use crate::prelude::TheRGBABuffer;
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, RenderError, Result, wgpu};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use theframework::{
    paint::{ImageHandle, PaintList},
    render_wgpu::UiImageCache,
    theui::paint::{UiImageResources, UiPaintError},
};
use wgpu::util::DeviceExt;

struct Entry {
    image: Arc<GpuImage>,
    handle: ImageHandle,
    used: u64,
}
#[derive(Default, Clone, Copy, Debug)]
pub(crate) struct IconStats {
    pub(crate) builds: u64,
    pub(crate) hits: u64,
    pub(crate) evictions: u64,
    pub(crate) resident_bytes: u64,
    pub(crate) resident_images: usize,
    pub(crate) pinned_bytes: u64,
    pub(crate) over_budget: bool,
}
pub(crate) struct GpuIcons {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    entries: HashMap<[u32; 24], Entry>,
    registry: Option<u64>,
    clock: u64,
    byte_budget: u64,
    entry_budget: usize,
    stats: IconStats,
}
impl GpuIcons {
    pub(crate) fn new(
        context: Arc<GpuContext>,
        byte_budget: u64,
        entry_budget: usize,
    ) -> Result<Self> {
        if context.is_lost() {
            return Err(RenderError("procedural icon device lost".into()));
        }
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("editor-icon-params"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("editor-icon-pipeline-layout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("editor-icon-source"),
                source: wgpu::ShaderSource::Wgsl(include_str!("procedural.wgsl").into()),
            });
        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("editor-icon-source"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
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
            layout,
            pipeline,
            entries: HashMap::new(),
            registry: None,
            clock: 0,
            byte_budget,
            entry_budget,
            stats: IconStats::default(),
        })
    }
    #[cfg(test)]
    pub(crate) fn stats(&self) -> IconStats {
        self.stats
    }
    fn domain(&mut self, registry: &ImageRegistry) -> Result<()> {
        if registry.context().generation() != self.context.generation() || self.context.is_lost() {
            return Err(RenderError(
                "procedural icon registry belongs to another GPU device".into(),
            ));
        }
        if self.registry != Some(registry.owner_id()) {
            // Old handles remain scoped to their old registry; never remove a
            // similarly numbered entry from the replacement registry.
            self.entries.clear();
            self.stats.pinned_bytes = 0;
            self.registry = Some(registry.owner_id());
            self.account();
        }
        Ok(())
    }
    fn account(&mut self) {
        self.stats.resident_bytes = self
            .entries
            .values()
            .map(|e| u64::from(e.image.descriptor.width) * u64::from(e.image.descriptor.height) * 4)
            .sum();
        self.stats.resident_images = self.entries.len();
        self.stats.over_budget =
            self.stats.resident_bytes > self.byte_budget || self.entries.len() > self.entry_budget;
    }
    pub(crate) fn prepare(
        &mut self,
        registry: &mut ImageRegistry,
        source: &ProceduralIcon,
    ) -> Result<ImageHandle> {
        self.domain(registry)?;
        self.clock = self
            .clock
            .checked_add(1)
            .expect("icon source clock exhausted");
        let key = source.words();
        if let Some(entry) = self.entries.get_mut(&key)
            && registry.get(entry.handle).is_some()
        {
            entry.used = self.clock;
            self.stats.hits += 1;
            return Ok(entry.handle);
        }
        let size = source.size();
        let image = GpuImage::new(
            &self.context,
            ImageDescriptor {
                format: wgpu::TextureFormat::Rgba8Unorm,
                ..ImageDescriptor::rgba(size, size)
            },
        )?;
        let bytes = key
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<_>>();
        let buffer = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("editor-icon-source-parameters"),
                contents: &bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.context.record_upload(bytes.len() as u64);
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("editor-icon-source-parameters"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("editor-icon-source"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("editor-icon-source"),
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
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.context.queue.submit([encoder.finish()]);
        let handle = registry.insert(image.clone())?;
        self.entries.insert(
            key,
            Entry {
                image,
                handle,
                used: self.clock,
            },
        );
        self.stats.builds += 1;
        self.account();
        Ok(handle)
    }
    /// Finalize only after all UI and tool/world contributions are assembled.
    /// Current/retained root handles pin exact source versions even over budget.
    pub(crate) fn finish_frame(
        &mut self,
        registry: &mut ImageRegistry,
        list: &PaintList,
    ) -> Result<()> {
        self.domain(registry)?;
        let pinned = list.image_handles().collect::<HashSet<_>>();
        self.account();
        if self.stats.over_budget {
            let mut candidates = self
                .entries
                .iter()
                .filter(|(_, e)| !pinned.contains(&e.handle))
                .map(|(key, e)| (e.used, *key))
                .collect::<Vec<_>>();
            candidates.sort_unstable();
            for (_, key) in candidates {
                if !self.stats.over_budget {
                    break;
                }
                let entry = self.entries.remove(&key).unwrap();
                registry.remove(entry.handle);
                self.stats.evictions += 1;
                self.account();
            }
        }
        self.stats.pinned_bytes = self
            .entries
            .values()
            .filter(|e| pinned.contains(&e.handle))
            .map(|e| u64::from(e.image.descriptor.width) * u64::from(e.image.descriptor.height) * 4)
            .sum();
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn clear(&mut self, registry: &mut ImageRegistry) {
        for entry in self.entries.values() {
            registry.remove(entry.handle);
        }
        self.entries.clear();
        self.registry = None;
        self.stats.pinned_bytes = 0;
        self.account();
    }
    pub(crate) fn frame<'a>(
        &'a mut self,
        cache: &'a mut UiImageCache,
        registry: &'a mut ImageRegistry,
    ) -> IconFrameImages<'a> {
        IconFrameImages {
            icons: self,
            cache,
            registry,
        }
    }
}
pub(crate) struct IconFrameImages<'a> {
    icons: &'a mut GpuIcons,
    cache: &'a mut UiImageCache,
    registry: &'a mut ImageRegistry,
}
#[cfg(test)]
impl IconFrameImages<'_> {
    pub(crate) fn finish_frame(
        &mut self,
        list: &PaintList,
    ) -> std::result::Result<(), UiPaintError> {
        self.cache.finish_frame(self.registry, list)?;
        self.icons
            .finish_frame(self.registry, list)
            .map_err(|e| UiPaintError(e.to_string()))
    }
}
impl UiImageResources for IconFrameImages<'_> {
    fn image(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> std::result::Result<ImageHandle, UiPaintError> {
        self.cache.frame(self.registry).image(key, source, changed)
    }
    fn asset(
        &mut self,
        key: &str,
        source: &theframework_core::ImageAsset,
        changed: bool,
    ) -> std::result::Result<ImageHandle, UiPaintError> {
        self.cache.frame(self.registry).asset(key, source, changed)
    }
    fn copied_framebuffer(
        &mut self,
        key: &str,
        source: &TheRGBABuffer,
        changed: bool,
    ) -> std::result::Result<ImageHandle, UiPaintError> {
        self.cache
            .frame(self.registry)
            .copied_framebuffer(key, source, changed)
    }
}
impl OverlayImages for IconFrameImages<'_> {
    fn procedural(
        &mut self,
        source: &ProceduralIcon,
    ) -> std::result::Result<ImageHandle, UiPaintError> {
        self.icons
            .prepare(self.registry, source)
            .map_err(|e| UiPaintError(e.to_string()))
    }
}
#[cfg(test)]
mod tests;
