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
    fn current(&self, key: &FrameKey) -> Option<OrthographicBakeOutput> {
        self.complete
            .as_ref()
            .filter(|old| old.compatible(key))
            .and(self.target.clone())
    }
    pub(super) fn advance_paged(
        &mut self,
        source: &OrthographicBakeSource<'_>,
        key: FrameKey,
        visible: Vec<((i32, i32), [u32; 4])>,
        tile_bytes: u64,
    ) -> Result<Option<OrthographicBakeOutput>> {
        if self.paging.is_none() {
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
                    ready.push((at, dispatch, lease))
                }
                rusterix::orthographic_bake::OrthographicBakeTilePoll::Missing => (),
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
            Self::pack(lease.source(), pixels_per_tile, &mut pixels)?;
            draws.push((at, dispatch, offset));
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
        if job.next == 0 {
            self.dispatch(&mut encoder, &job.output, &params, &self.dummy, true);
        }
        for (at, dispatch, offset) in draws {
            params.tile[0] = at.0;
            params.tile[1] = at.1;
            params.dispatch = dispatch;
            params.bits[2] = offset;
            self.dispatch(&mut encoder, &job.output, &params, buffer.buffer(), false);
        }
        job.next = end;
        let done = Arc::new(AtomicBool::new(false));
        job.pending = Some(done.clone());
        let pin = buffer.pin();
        self.context.queue.submit(Some(encoder.finish()));
        self.context.queue.on_submitted_work_done(move || {
            drop(pin);
            drop(submission);
            done.store(true, Ordering::Release);
        });
        self.paging = Some(job);
        Ok(self.current(&key))
    }
}
