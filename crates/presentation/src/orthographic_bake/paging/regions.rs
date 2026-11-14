//! Stream bounded original rectangles into the same private projection target.
//! The shader still computes exact original tile/pixel coordinates; region
//! metadata only translates those coordinates into the currently resident page.
use super::*;
use rusterix::orthographic_bake::{OrthographicBakeTilePoll, region_decode::OriginalRegion};

impl Paging {
    fn next_region(&mut self, source: &OrthographicBakeSource<'_>) {
        let side = source.asset().tile_size;
        self.region_origin[0] += OriginalRegion::MAX_SIDE.min(side - self.region_origin[0]);
        if self.region_origin[0] == side {
            self.region_origin[0] = 0;
            self.region_origin[1] += OriginalRegion::MAX_SIDE.min(side - self.region_origin[1]);
            if self.region_origin[1] == side {
                let at = self.visible[self.next].0;
                if let Some(revision) = source.tile_revision(at.0, at.1) {
                    self.revisions.insert(at, revision);
                }
                self.region_origin = [0; 2];
                self.next += 1;
            }
        }
    }
    fn region_dispatch(&self, at: (i32, i32), region: OriginalRegion) -> [u32; 4] {
        let p = &self.key.params;
        let exact = p.tile[2] != 0;
        let origin = if exact {
            [p.bounds[0], p.bounds[2]]
        } else {
            [0.; 2]
        };
        let tile_lo = [
            origin[0] + at.0 as f32 * p.source[1],
            origin[1] + at.1 as f32 * p.source[1],
        ];
        let lo_u = tile_lo[0] + region.origin[0] as f32 / p.source[0];
        let hi_u = tile_lo[0] + (region.origin[0] + region.size[0]) as f32 / p.source[0];
        let hi_v = tile_lo[1] + p.source[1] - region.origin[1] as f32 / p.source[0];
        let lo_v =
            tile_lo[1] + p.source[1] - (region.origin[1] + region.size[1]) as f32 / p.source[0];
        // Conservative destination bounds, with exact original pixel membership
        // checked in WGSL. Never resample/crop in a different camera space.
        let x = |u| ((u - p.plane[0]) / p.plane[2] + 1.) * p.size[0] as f32 * 0.5;
        let y = |v| (1. - (v - p.plane[1]) / p.plane[3]) * p.size[1] as f32 * 0.5;
        // Cancellation at large world coordinates can magnify f32 error after
        // projection. Expand by the arithmetic scale, falling back to the
        // original tile's conservative bounds when precision is exhausted.
        let scale = origin
            .into_iter()
            .chain(tile_lo)
            .chain(p.plane)
            .chain([p.source[1]])
            .map(f32::abs)
            .sum::<f32>();
        let error = scale * (64. * f32::EPSILON);
        let pad_x = 4. + error / p.plane[2] * p.size[0] as f32;
        let pad_y = 4. + error / p.plane[3] * p.size[1] as f32;
        if !pad_x.is_finite() || !pad_y.is_finite() {
            return self.visible[self.next].1;
        }
        [
            (x(lo_u).floor() - pad_x).clamp(0., p.size[0] as f32) as u32,
            (y(hi_v).floor() - pad_y).clamp(0., p.size[1] as f32) as u32,
            (x(hi_u).ceil() + pad_x).clamp(0., p.size[0] as f32) as u32,
            (y(lo_v).ceil() + pad_y).clamp(0., p.size[1] as f32) as u32,
        ]
    }
}
impl OrthographicBakeRenderer {
    pub(super) fn advance_region_page(
        &mut self,
        source: &OrthographicBakeSource<'_>,
        key: FrameKey,
        mut job: Paging,
    ) -> Result<Option<OrthographicBakeOutput>> {
        // Bound metadata work too: a highly zoomed viewport may skip many
        // nonvisible original pages without decoding or uploading any of them.
        for _ in 0..64 {
            if job.next == job.visible.len() {
                self.paging = Some(job);
                return Ok(self.current(&key));
            }
            let at = job.visible[job.next].0;
            let side = source.asset().tile_size;
            let region = OriginalRegion {
                origin: job.region_origin,
                size: [
                    OriginalRegion::MAX_SIDE.min(side - job.region_origin[0]),
                    OriginalRegion::MAX_SIDE.min(side - job.region_origin[1]),
                ],
            };
            let dispatch = job.region_dispatch(at, region);
            if dispatch[0] >= dispatch[2] || dispatch[1] >= dispatch[3] {
                job.next_region(source);
                continue;
            }
            let lease = match source
                .poll_region(at.0, at.1, region)
                .map_err(RenderError)?
            {
                OrthographicBakeTilePoll::Ready(lease) => Some(lease),
                OrthographicBakeTilePoll::Missing => None,
                OrthographicBakeTilePoll::Pending => {
                    self.paging = Some(job);
                    return Ok(self.current(&key));
                }
            };
            let mut pixels = Vec::new();
            if let Some(lease) = &lease {
                let count = region.validate(side).map_err(RenderError)?;
                pixels.reserve_exact(count);
                Self::pack(lease.source(), count, &mut pixels)?;
            }
            let bytes = (pixels.len() * std::mem::size_of::<Pixel>()) as u64;
            let Some(submission) = self.context.try_source_submission(bytes) else {
                self.paging = Some(job);
                return Ok(self.current(&key));
            };
            let buffer = self
                .page_buffer
                .upgrade()
                .filter(|buffer| buffer.bytes() >= PAGE_BYTES)
                .or_else(|| self.context.try_source_buffer(PAGE_BYTES, None));
            let Some(buffer) = buffer else {
                self.paging = Some(job);
                return Ok(self.current(&key));
            };
            self.page_buffer = Arc::downgrade(&buffer);
            self.context.touch_source_buffer(&buffer);
            if bytes > 0 {
                self.context
                    .queue
                    .write_buffer(buffer.buffer(), 0, bytemuck::cast_slice(&pixels));
                self.context.record_upload(bytes);
            }
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("original bake region page"),
                    });
            let mut uniforms = job.initialize(self, &mut encoder)?;
            let mut params = job.key.params;
            params.tile[0] = at.0;
            params.tile[1] = at.1;
            params.dispatch = dispatch;
            params.region = [
                region.origin[0],
                region.origin[1],
                region.size[0],
                region.size[1],
            ];
            params.bits[2] = 0;
            params.bits[3] = u32::from(lease.is_none());
            uniforms.push(self.dispatch(
                &mut encoder,
                &job.output,
                &params,
                buffer.buffer(),
                false,
            )?);
            job.next_region(source);
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
            return Ok(self.current(&key));
        }
        self.paging = Some(job);
        Ok(self.current(&key))
    }
}
