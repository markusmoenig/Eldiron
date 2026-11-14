//! Asynchronous, bounded asset depth capture. The displayed scene keeps no
//! staging storage; a job owns and drops its explicit request independently.
use super::*;
use std::sync::mpsc;
use vek::Vec3;

pub struct CapturedSurfaceDepth {
    pub size: (u32, u32),
    pub revision: u64,
    pub selection: SurfaceSelection,
    /// Camera-linear values, infinity where the GPU surface has no geometry.
    pub depth: Vec<f32>,
}
pub struct SurfaceDepthCapture {
    buffer: wgpu::Buffer,
    padded: u32,
    source: SurfaceOutput,
    ready: mpsc::Receiver<std::result::Result<(), wgpu::BufferAsyncError>>,
    finished: bool,
}
impl SurfaceOutput {
    /// Request from a fresh surface output before any later producer query can
    /// reuse its targets. The copy is submitted immediately and survives reuse.
    /// Only an explicit bake/export job should request asset depth. The caller
    /// owns queue bounds and validates its document/source revision on completion.
    pub fn capture_asset_depth(&self, context: &GpuContext) -> Result<SurfaceDepthCapture> {
        if context.is_lost() || self.generation != context.generation() {
            return Err(RenderError(
                "asset depth belongs to a lost or different device".into(),
            ));
        }
        let padded = (self.size.0 * 16).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let plane = u64::from(padded) * u64::from(self.size.1);
        let bytes = plane
            .checked_mul(2)
            .filter(|bytes| {
                *bytes <= 64 * 1024 * 1024 && *bytes <= context.device.limits().max_buffer_size
            })
            .ok_or_else(|| RenderError("asset depth exceeds 64 MiB staging budget".into()))?;
        let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("requested asset surface depth"),
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("asset surface depth capture"),
            });
        for (texture, offset) in [(&self.world_texture, 0), (&self.metadata_texture, plane)] {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset,
                        bytes_per_row: Some(padded),
                        rows_per_image: Some(self.size.1),
                    },
                },
                texture.size(),
            );
        }
        context.record_asset_bake(u64::from(self.size.0) * u64::from(self.size.1) * 32);
        context.queue.submit(Some(encoder.finish()));
        let (send, ready) = mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        Ok(SurfaceDepthCapture {
            buffer,
            padded,
            source: self.clone(),
            ready,
            finished: false,
        })
    }
}
impl SurfaceDepthCapture {
    /// Poll once; never wait for GPU completion or run a blocking map loop.
    pub fn try_finish(&mut self, context: &GpuContext) -> Result<Option<CapturedSurfaceDepth>> {
        if self.finished {
            return Err(RenderError("asset depth already consumed".into()));
        }
        if context.is_lost() || self.source.generation != context.generation() {
            return Err(RenderError(
                "asset depth invalidated by device replacement/loss".into(),
            ));
        }
        context.poll()?;
        match self.ready.try_recv() {
            Err(mpsc::TryRecvError::Empty) => return Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(RenderError("asset depth callback disconnected".into()));
            }
            Ok(result) => {
                result.map_err(|e| RenderError(format!("asset depth mapping failed: {e}")))?
            }
        }
        let size = self.source.size;
        let mapped = self.buffer.get_mapped_range(..);
        let plane = self.padded as usize * size.1 as usize;
        let value = |offset| f32::from_ne_bytes(mapped[offset..offset + 4].try_into().unwrap());
        let mut depth = vec![f32::INFINITY; size.0 as usize * size.1 as usize];
        for y in 0..size.1 as usize {
            for x in 0..size.0 as usize {
                let base = y * self.padded as usize + x * 16;
                let face = value(plane + base + 8);
                if value(plane + base + 12) > 0.5 && face.is_finite() && face >= 0. {
                    let world = Vec3::new(value(base), value(base + 4), value(base + 8));
                    depth[y * size.0 as usize + x] =
                        (world - self.source.camera.pos).dot(self.source.camera.forward);
                }
            }
        }
        drop(mapped);
        self.buffer.unmap();
        self.finished = true;
        Ok(Some(CapturedSurfaceDepth {
            size,
            revision: self.source.revision,
            selection: self.source.selection,
            depth,
        }))
    }
}
impl Drop for SurfaceDepthCapture {
    fn drop(&mut self) {
        if !self.finished {
            self.buffer.unmap();
        }
    }
}
