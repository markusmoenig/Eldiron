//! Cooperative original-channel uploads into one reusable source page. Only a
//! completed private target is published; projection/source changes cancel it.
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, PartialEq)]
pub(super) struct FrameKey {
    pub revision: Uuid,
    pub request: Option<(Uuid, Uuid)>,
    pub params: Params,
}
impl FrameKey {
    fn projection(&self) -> Self {
        let mut key = self.clone();
        key.params.sun = [0.; 4];
        key.params.sun_color = [0.; 4];
        key.params.tile[3] = 0;
        key
    }
    fn compatible(&self, other: &Self) -> bool {
        let mut a = self.projection();
        let mut b = other.projection();
        if a.request.is_some() && a.request == b.request {
            // A progressive view may finish its current visible subset while
            // later samples/tiles arrive. Geometry and projection are frozen by
            // the addressed producer request; final committed sources use exact
            // publication revisions again.
            a.revision = Uuid::nil();
            b.revision = Uuid::nil();
        }
        a == b
    }
}
pub(super) struct Paging {
    key: FrameKey,
    output: OrthographicBakeOutput,
    visible: Vec<((i32, i32), [u32; 4])>,
    next: usize,
    pending: Option<Arc<AtomicBool>>,
    base: Option<OrthographicBakeOutput>,
    revisions: HashMap<(i32, i32), Uuid>,
}
impl OrthographicBakeRenderer {
    pub(super) fn cancel_incompatible(&mut self, key: &FrameKey) {
        if self
            .paging
            .as_ref()
            .is_some_and(|job| !job.key.compatible(key))
        {
            let job = self.paging.take().unwrap();
            if job
                .pending
                .as_ref()
                .is_some_and(|done| !done.load(Ordering::Acquire))
            {
                // Its queued commands still read this allocation. A new
                // projection gets another page until the old GPU lease retires.
                self.page_buffer = Weak::new();
            }
        }
    }
    pub(super) fn current(&self, key: &FrameKey) -> Option<OrthographicBakeOutput> {
        self.complete
            .as_ref()
            .filter(|old| old.compatible(key))
            .and(self.target.clone())
    }
    pub(super) fn advance_paged(
        &mut self,
        source: &OrthographicBakeSource<'_>,
        key: FrameKey,
        mut visible: Vec<((i32, i32), [u32; 4])>,
        tile_bytes: u64,
    ) -> Result<Option<OrthographicBakeOutput>> {
        if self.paging.is_none() {
            let desired: HashMap<_, _> = visible
                .iter()
                .filter_map(|(at, _)| {
                    source
                        .tile_revision(at.0, at.1)
                        .map(|revision| (*at, revision))
                })
                .collect();
            // Only a live producer request fixes geometry/projection while tile
            // samples advance. Lighting, camera, bounds, size and request must
            // all match exactly to reuse prior pixels/depth. Removed coordinates
            // force a full rebuild rather than leaving inherited pixels behind.
            let base = self
                .complete
                .as_ref()
                .filter(|old| {
                    key.request.is_some()
                        && old.request == key.request
                        && old.params == key.params
                        && self
                            .complete_tiles
                            .keys()
                            .all(|at| desired.contains_key(at))
                })
                .and(self.target.clone());
            let revisions = if base.is_some() {
                visible.retain(|(at, _)| self.complete_tiles.get(at) != desired.get(at));
                if visible.is_empty() {
                    self.complete = Some(key.clone());
                    self.complete_tiles = desired;
                    let target = self.target.as_mut().unwrap();
                    target.source_revision = key.revision;
                    self.waiting = false;
                    return Ok(Some(target.clone()));
                }
                self.complete_tiles.clone()
            } else {
                HashMap::new()
            };
            let previous = self.target.take();
            let output = self.target(
                (key.params.size[0], key.params.size[1]),
                key.revision,
                key.params.size[3] != 0,
                key.request,
            );
            self.target = previous;
            self.paging = Some(Paging {
                key: key.clone(),
                output: output?,
                visible,
                next: 0,
                pending: None,
                base,
                revisions,
            });
        }
        let mut job = self.paging.take().unwrap();
        if job
            .pending
            .as_ref()
            .is_some_and(|done| !done.load(Ordering::Acquire))
        {
            self.paging = Some(job);
            return Ok(self.current(&key));
        }
        job.pending = None;
        if job.next == job.visible.len() {
            self.waiting = job.key != key;
            self.complete = Some(job.key);
            self.complete_tiles = job.revisions;
            self.target = Some(job.output.clone());
            return Ok(Some(job.output));
        }
        if self.context.source_pool_usage().submissions
            >= self.context.source_pool_usage().submission_limit
        {
            self.paging = Some(job);
            return Ok(self.current(&key));
        }
        let capacity = PAGE_BYTES.max(tile_bytes);
        let buffer = self
            .page_buffer
            .upgrade()
            .filter(|buffer| buffer.bytes() >= capacity)
            .or_else(|| self.context.try_source_buffer(capacity, None));
        let Some(buffer) = buffer else {
            self.paging = Some(job);
            return Ok(self.current(&key));
        };
        self.page_buffer = Arc::downgrade(&buffer);
        self.context.touch_source_buffer(&buffer);
        let count = (buffer.bytes() / tile_bytes).min(64) as usize;
        let end = (job.next + count).min(job.visible.len());
        let pixels_per_tile = (tile_bytes / std::mem::size_of::<Pixel>() as u64) as usize;
        // Resolve the complete page before packing/uploading. Pending decode
        // never becomes an omitted tile or a partially published projection.
        let mut ready = Vec::new();
        for &(at, dispatch) in &job.visible[job.next..end] {
            match source.poll_tile(at.0, at.1).map_err(RenderError)? {
                rusterix::orthographic_bake::OrthographicBakeTilePoll::Ready(lease) => {
                    ready.push((at, dispatch, Some(lease)))
                }
                rusterix::orthographic_bake::OrthographicBakeTilePoll::Missing => {
                    ready.push((at, dispatch, None))
                }
                rusterix::orthographic_bake::OrthographicBakeTilePoll::Pending => {
                    self.paging = Some(job);
                    return Ok(self.current(&key));
                }
            }
        }
        let mut pixels = Vec::with_capacity(ready.len() * pixels_per_tile);
        let mut draws = Vec::new();
        for (at, dispatch, lease) in ready {
            let offset = pixels.len() as u32;
            if let Some(lease) = &lease {
                Self::pack(lease.source(), pixels_per_tile, &mut pixels)?;
            }
            draws.push((at, dispatch, offset, lease.is_none()));
            if let Some(revision) = source.tile_revision(at.0, at.1) {
                job.revisions.insert(at, revision);
            }
        }
        let upload_bytes = (pixels.len() * std::mem::size_of::<Pixel>()) as u64;
        let Some(submission) = self.context.try_source_submission(upload_bytes) else {
            self.paging = Some(job);
            return Ok(self.current(&key));
        };
        if upload_bytes != 0 {
            self.context
                .queue
                .write_buffer(buffer.buffer(), 0, bytemuck::cast_slice(&pixels));
            self.context.record_upload(upload_bytes);
        }
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("orthographic bake source page"),
                });
        let mut params = job.key.params;
        let mut uniforms = Vec::new();
        if job.next == 0 {
            if let Some(base) = job.base.take() {
                encoder.copy_texture_to_texture(
                    base.image.texture.as_image_copy(),
                    job.output.image.texture.as_image_copy(),
                    wgpu::Extent3d {
                        width: base.size.0,
                        height: base.size.1,
                        depth_or_array_layers: 1,
                    },
                );
                encoder.copy_buffer_to_buffer(
                    base.depth(),
                    0,
                    job.output.depth(),
                    0,
                    u64::from(base.size.0) * u64::from(base.size.1) * 4,
                );
            } else {
                uniforms.push(self.dispatch(&mut encoder, &job.output, &params, &self.dummy, true)?);
            }
        }
        for (at, dispatch, offset, missing) in draws {
            params.tile[0] = at.0;
            params.tile[1] = at.1;
            params.dispatch = dispatch;
            params.bits[2] = offset;
            params.bits[3] = u32::from(missing);
            uniforms.push(self.dispatch(&mut encoder, &job.output, &params, buffer.buffer(), false)?);
        }
        job.next = end;
        let done = Arc::new(AtomicBool::new(false));
        job.pending = Some(done.clone());
        let pin = buffer.pin();
        self.context.queue.submit(Some(encoder.finish()));
        drop(uniforms);
        self.context.queue.on_submitted_work_done(move || {
            drop(pin);
            drop(submission);
            done.store(true, Ordering::Release);
        });
        self.paging = Some(job);
        Ok(self.current(&key))
    }
}
