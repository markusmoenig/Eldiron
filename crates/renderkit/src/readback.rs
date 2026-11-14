use crate::{GpuContext, GpuImage, ImageAsset, RenderError, Result};
use std::sync::{Arc, mpsc};
/// An explicit capture request. Mapping is asynchronous; ordinary rendering never
/// creates a ticket or allocates staging memory.
pub struct ReadbackTicket {
    buffer: wgpu::Buffer,
    image: Arc<GpuImage>,
    padded: u32,
    ready: mpsc::Receiver<std::result::Result<(), wgpu::BufferAsyncError>>,
    device_generation: u64,
    finished: bool,
    lease: Option<crate::CaptureLease>,
}
impl ReadbackTicket {
    pub fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Result<Self> {
        Self::capture_for(context, image, false)
    }
    /// Explicit original-image asset production, separately labeled from
    /// viewport/export requests. One ticket is bounded to 64 MiB of staging.
    pub fn capture_asset(context: &GpuContext, image: Arc<GpuImage>) -> Result<Self> {
        Self::capture_for(context, image, true)
    }
    fn capture_for(context: &GpuContext, image: Arc<GpuImage>, asset: bool) -> Result<Self> {
        if context.is_lost() {
            return Err(RenderError("capture device is lost".into()));
        }
        if image.device_generation() != context.generation() {
            return Err(RenderError("capture belongs to another device".into()));
        }
        if !matches!(
            image.descriptor.format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(RenderError("capture requires an RGBA8/BGRA8 output".into()));
        }
        let padded = (image.descriptor.width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let bytes = u64::from(padded) * u64::from(image.descriptor.height);
        if asset && bytes > 64 * 1024 * 1024 {
            return Err(RenderError(
                "asset capture exceeds 64 MiB staging budget".into(),
            ));
        }
        let lease = context.reserve_capture(bytes)?;
        let logical = u64::from(image.descriptor.width) * u64::from(image.descriptor.height) * 4;
        if asset {
            context.record_asset_bake(logical);
        } else {
            context.record_capture(logical);
        }
        let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(if asset {
                "renderkit-requested-asset-readback"
            } else {
                "renderkit-requested-readback"
            }),
            size: padded as u64 * image.descriptor.height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(if asset {
                    "renderkit-asset-capture"
                } else {
                    "renderkit-capture"
                }),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &image.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(image.descriptor.height),
                },
            },
            image.texture.size(),
        );
        context.queue.submit([encoder.finish()]);
        lease.retain_for_submission(&context.queue);
        let (tx, ready) = mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        Ok(Self {
            buffer,
            image,
            padded,
            ready,
            device_generation: context.generation(),
            finished: false,
            lease: Some(lease),
        })
    }
    pub fn try_finish(&mut self, context: &GpuContext) -> Result<Option<ImageAsset>> {
        if self.finished {
            return Err(RenderError("capture already consumed".into()));
        }
        if self.device_generation != context.generation() {
            return Err(RenderError(
                "capture invalidated by device replacement".into(),
            ));
        }
        if context.is_lost() {
            return Err(RenderError("capture device is lost".into()));
        }
        context.poll()?;
        // Poll can deliver loss after a successful map notification was already
        // queued. That notification no longer authorizes mapped-buffer access.
        if context.is_lost() {
            return Err(RenderError("capture device is lost".into()));
        }
        match self.ready.try_recv() {
            Err(mpsc::TryRecvError::Empty) => return Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(RenderError("capture callback disconnected".into()));
            }
            Ok(result) => {
                result.map_err(|e| RenderError(format!("capture mapping failed: {e}")))?
            }
        }
        let d = self.image.descriptor;
        let mapped = self.buffer.get_mapped_range(..);
        let mut pixels = Vec::with_capacity(d.width as usize * d.height as usize * 4);
        for row in mapped.chunks_exact(self.padded as usize) {
            pixels.extend_from_slice(&row[..d.width as usize * 4]);
        }
        drop(mapped);
        self.buffer.unmap();
        self.buffer.destroy();
        self.finished = true;
        self.lease = None;
        if matches!(
            d.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        let mut result =
            ImageAsset::rgba(d.width, d.height, pixels).map_err(|e| RenderError(e.to_string()))?;
        result.color_space = d.color_space;
        result.alpha_mode = d.alpha_mode;
        Ok(Some(result))
    }
}
impl Drop for ReadbackTicket {
    fn drop(&mut self) {
        if !self.finished {
            // This staging buffer is exclusive to the ticket. destroy cancels
            // a pending map and is idempotent after device loss; unmap validates
            // a live buffer and can panic after its device destroyed it.
            self.buffer.destroy();
        }
    }
}
