//! Private replacement buffers permit cooperative uploads without exposing a
//! partially written source to rendering, picking or the last presented image.
use super::*;
use crate::source_upload::SourceUploadBudget;

enum Data {
    Header([u32; 5]),
    Words(Arc<Vec<u32>>),
    Surface(PaintEntryStorage),
}
impl Data {
    fn words(&self) -> &[u32] {
        match self {
            Self::Header(words) => words,
            Self::Words(words) => words,
            Self::Surface(entries) => bytemuck::cast_slice(entries),
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Header(a), Self::Header(b)) => a == b,
            (Self::Words(a), Self::Words(b)) => Arc::ptr_eq(a, b),
            (Self::Surface(a), Self::Surface(b)) => a.same_storage(b),
            _ => false,
        }
    }
}
struct Span {
    data: Data,
    cursor: usize,
}
pub(crate) struct Pending {
    buffer: wgpu::Buffer,
    spans: Vec<Span>,
    next: usize,
    compute: bool,
}
impl Pending {
    fn matches_layout(&self, compute: bool, data: &[Data]) -> bool {
        self.compute == compute
            && self.spans.len() == data.len()
            && self
                .spans
                .iter()
                .zip(data)
                .all(|(a, b)| a.data.words().len() == b.words().len())
    }
    fn update(&mut self, data: Vec<Data>) {
        for (span, data) in self.spans.iter_mut().zip(data) {
            if !span.data.same(&data) {
                span.data = data;
                span.cursor = 0;
            }
        }
    }
    fn advance(&mut self, queue: &wgpu::Queue, budget: &mut SourceUploadBudget) -> bool {
        while self
            .spans
            .iter()
            .any(|span| span.cursor < span.data.words().len())
        {
            let available = budget.available() / 4;
            if available == 0 {
                return false;
            }
            let index = self.next;
            self.next = (self.next + 1) % self.spans.len();
            let offset: usize = self.spans[..index]
                .iter()
                .map(|span| span.data.words().len())
                .sum();
            let span = &mut self.spans[index];
            let words = span.data.words();
            let end = (span.cursor + available.min(16 * 1024)).min(words.len());
            if end == span.cursor {
                continue;
            }
            queue.write_buffer(
                &self.buffer,
                (offset + span.cursor) as u64 * 4,
                bytemuck::cast_slice(&words[span.cursor..end]),
            );
            budget.record((end - span.cursor) * 4);
            span.cursor = end;
        }
        true
    }
}
impl VM {
    pub(crate) fn prepare_organic_uploads(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        budget: &mut SourceUploadBudget,
    ) -> crate::SceneVMResult<bool> {
        if !self.enabled
            || self.frozen_frame_texture.is_some()
            || !matches!(
                self.render_mode,
                RenderMode::Raster3D | RenderMode::Compute3D
            )
        {
            self.cancel_organic_preparation();
            if let Some(gpu) = &mut self.gpu {
                gpu.pending_source_upload = None;
            }
            return Ok(true);
        }
        let compute = self.render_mode == RenderMode::Compute3D;
        let limit = device
            .limits()
            .max_buffer_size
            .min(u64::from(device.limits().max_storage_buffer_binding_size));
        let surface_bytes = if compute {
            20u64.saturating_add(
                self.paint_entries()
                    .filter(|entries| !entries.is_empty())
                    .map_or(size_of::<Raster3DSurfacePaintEntry>() as u64, |entries| {
                        size_of_val(entries) as u64
                    }),
            )
        } else {
            0
        };
        if self
            .compute_organic_source_bytes()
            .saturating_add(surface_bytes)
            > limit
        {
            return Err(crate::SceneVMError::BufferAllocationFailed(
                "live organic source exceeds device binding limit".into(),
            ));
        }
        let Some(organic) = self.poll_organic_words(budget, limit.saturating_sub(surface_bytes))?
        else {
            if let Some(gpu) = &mut self.gpu {
                gpu.pending_source_upload = None;
            }
            return Ok(false);
        };
        if self.gpu.is_none() {
            self.init_gpu(device)?;
        }
        let surface = if compute && self.paged_surface_paint.is_some() {
            Some(self.paged_surface_paint.as_ref().unwrap().entries.clone())
        } else if compute {
            let dummy = Raster3DSurfacePaintEntry::zeroed();
            let current = self
                .paint_entries()
                .filter(|entries| !entries.is_empty())
                .unwrap_or(std::slice::from_ref(&dummy));
            let gpu = self.gpu.as_ref().unwrap();
            let pending = gpu
                .pending_source_upload
                .as_ref()
                .filter(|pending| pending.compute)
                .and_then(|pending| match &pending.spans[1].data {
                    Data::Surface(data) => Some(data),
                    _ => None,
                });
            let old = pending.or(gpu.compute3d_aux_surface.as_ref());
            Some(match old {
                Some(old) if old.as_ref() == current => old.clone(),
                _ => PaintEntryStorage::copy_from(current),
            })
        } else {
            None
        };
        let gpu = self.gpu.as_mut().unwrap();
        let previous = if compute {
            gpu.compute3d_aux_organic.as_ref()
        } else {
            gpu.organic_billboard_words.as_ref()
        };
        let same_surface = !compute
            || gpu
                .compute3d_aux_surface
                .as_ref()
                .zip(surface.as_ref())
                .is_some_and(|(a, b)| a.same_storage(b));
        let buffer_present = if compute {
            gpu.compute3d_aux_ssbo.is_some()
        } else {
            gpu.organic_billboard_ssbo.is_some()
        };
        let same_source = previous.is_some_and(|old| {
            Arc::ptr_eq(&old.sources, &organic.sources)
                && old.instances.len() == organic.instances.len()
        });
        if buffer_present && same_surface && same_source {
            if previous.is_some_and(|old| Arc::ptr_eq(&old.instances, &organic.instances)) {
                gpu.pending_source_upload = None;
                return Ok(true);
            }
            // The common camera-only case needs one prefix write, not a private
            // replacement or a second transfer of all retained sprite pixels.
            let bytes = organic.instances.len() * 4;
            if budget.available() >= bytes {
                let (buffer, offset) = if compute {
                    (
                        gpu.compute3d_aux_ssbo.as_ref().unwrap(),
                        20 + size_of_val(surface.as_ref().unwrap().as_ref()) as u64,
                    )
                } else {
                    (gpu.organic_billboard_ssbo.as_ref().unwrap(), 0)
                };
                queue.write_buffer(buffer, offset, bytemuck::cast_slice(&organic.instances));
                budget.record(bytes);
                if compute {
                    gpu.compute3d_aux_organic = Some(organic);
                } else {
                    gpu.organic_billboard_count = organic.instances[2];
                    gpu.organic_billboard_words = Some(organic);
                }
                gpu.pending_source_upload = None;
                return Ok(true);
            }
        }
        let mut data = Vec::new();
        if let Some(surface) = &surface {
            let offset = 4usize
                .checked_add(size_of_val(surface.as_ref()) / 4)
                .ok_or_else(|| {
                    crate::SceneVMError::BufferAllocationFailed("source offset overflow".into())
                })?;
            data.push(Data::Header([
                u32::MAX,
                4,
                surface.len() as u32,
                offset as u32,
                organic.len() as u32,
            ]));
            data.push(Data::Surface(surface.clone()));
        }
        data.push(Data::Words(organic.instances.clone()));
        data.push(Data::Words(organic.sources.clone()));
        let bytes = data
            .iter()
            .try_fold(0usize, |bytes, span| {
                bytes.checked_add(size_of_val(span.words()))
            })
            .ok_or_else(|| {
                crate::SceneVMError::BufferAllocationFailed("source size overflow".into())
            })?;
        if bytes as u64 > device.limits().max_buffer_size
            || bytes as u64 > u64::from(device.limits().max_storage_buffer_binding_size)
        {
            return Err(crate::SceneVMError::BufferAllocationFailed(
                "live organic source exceeds device binding limit".into(),
            ));
        }
        if gpu
            .pending_source_upload
            .as_ref()
            .is_none_or(|pending| !pending.matches_layout(compute, &data))
        {
            gpu.pending_source_upload = Some(Pending {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("private-live-scene-source"),
                    size: bytes as u64,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                spans: data
                    .into_iter()
                    .map(|data| Span { data, cursor: 0 })
                    .collect(),
                next: 0,
                compute,
            });
        } else {
            gpu.pending_source_upload.as_mut().unwrap().update(data);
        }
        if !gpu
            .pending_source_upload
            .as_mut()
            .unwrap()
            .advance(queue, budget)
        {
            return Ok(false);
        }
        let pending = gpu.pending_source_upload.take().unwrap();
        if compute {
            gpu.compute3d_aux_ssbo = Some(pending.buffer);
            gpu.compute3d_paint_table = None;
            gpu.compute3d_aux_ssbo_size = bytes;
            gpu.compute3d_aux_surface = surface;
            gpu.compute3d_aux_organic = Some(organic);
        } else {
            gpu.organic_billboard_ssbo = Some(pending.buffer);
            gpu.organic_billboard_ssbo_size = bytes;
            gpu.organic_billboard_count = organic.instances[2];
            gpu.organic_billboard_words = Some(organic);
        }
        Ok(true)
    }
}
