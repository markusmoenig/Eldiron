//! Bounded asynchronous GPU metadata. This never captures a color image.
use crate::{GpuContext, RenderError, Result};
use std::sync::mpsc;
/// A single four-byte status word copied by a producer's submitted encoder.
/// The callback transports readiness only; this owner reads/unmaps the buffer.
/// Dropping before/after completion safely cancels unread receipts. A new
/// request owns its own mailbox, so delayed callbacks cannot affect it.
pub struct StatusReadback {
    buffer: wgpu::Buffer,
    ready: mpsc::Receiver<std::result::Result<(), wgpu::BufferAsyncError>>,
    generation: u64,
    finished: bool,
}
impl StatusReadback {
    /// Takes ownership of exclusive staging; cancelling destroys that buffer.
    /// Call only after submitting the encoder that writes this status buffer.
    /// Device polling remains owned by the application host; this never waits.
    pub fn map(context: &GpuContext, buffer: wgpu::Buffer) -> Result<Self> {
        if context.is_lost() {
            return Err(RenderError("status readback device is lost".into()));
        }
        if buffer.size() != 4 || !buffer.usage().contains(wgpu::BufferUsages::MAP_READ) {
            return Err(RenderError(
                "status readback requires a four-byte MAP_READ buffer".into(),
            ));
        }
        let (send, ready) = mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        context.record_status_readback();
        Ok(Self {
            buffer,
            ready,
            generation: context.generation(),
            finished: false,
        })
    }
    pub fn try_finish(&mut self, context: &GpuContext) -> Result<Option<u32>> {
        if context.is_lost() {
            return Err(RenderError("status readback device is lost".into()));
        }
        if self.finished {
            return Err(RenderError("status readback already consumed".into()));
        }
        if self.generation != context.generation() {
            return Err(RenderError(
                "status readback invalidated by device replacement".into(),
            ));
        }
        // The common host already polls once per update. Reading a mailbox here
        // neither polls nor blocks the driver, including the not-ready path.
        match self.ready.try_recv() {
            Err(mpsc::TryRecvError::Empty) => return Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(RenderError("status callback disconnected".into()));
            }
            Ok(result) => {
                result.map_err(|error| RenderError(format!("status mapping failed: {error}")))?
            }
        }
        let mapped = self.buffer.get_mapped_range(..);
        let value = u32::from_le_bytes(mapped[..4].try_into().unwrap());
        drop(mapped);
        self.buffer.unmap();
        self.finished = true;
        Ok(Some(value))
    }
}
impl Drop for StatusReadback {
    fn drop(&mut self) {
        if !self.finished {
            // destroy is idempotent after device loss; unmapping an already
            // destroyed buffer is a validation error on native wgpu.
            self.buffer.destroy();
        }
    }
}
