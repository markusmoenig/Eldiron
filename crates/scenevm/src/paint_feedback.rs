//! A bounded four-byte receipt for authored paint dependencies. It contains no
//! color/depth image and belongs to the requesting producer, not a global inbox.
use renderkit::{GpuContext, RenderError, Result, StatusReadback, wgpu};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintPageStatus {
    Ready,
    MissingEntry(u32),
}
pub struct PaintPageReadback {
    status: StatusReadback,
    entries: u32,
}
impl PaintPageReadback {
    /// The caller must have submitted the compute pass that writes the prefix.
    pub(crate) fn capture(
        context: &GpuContext,
        source: &wgpu::Buffer,
        entries: u32,
    ) -> Result<Self> {
        if context.is_lost()
            || entries == 0
            || source.size() < 4
            || !source.usage().contains(wgpu::BufferUsages::COPY_SRC)
        {
            return Err(RenderError("invalid paint dependency status source".into()));
        }
        let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SceneVM paint dependency status"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = context.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, 4);
        context.queue.submit([encoder.finish()]);
        Ok(Self {
            status: StatusReadback::map(context, buffer)?,
            entries,
        })
    }
    pub fn try_finish(&mut self, context: &GpuContext) -> Result<Option<PaintPageStatus>> {
        self.status
            .try_finish(context)?
            .map(|value| {
                if value == u32::MAX {
                    Ok(PaintPageStatus::Ready)
                } else if value < self.entries {
                    Ok(PaintPageStatus::MissingEntry(value))
                } else {
                    Err(RenderError(
                        "paint dependency index exceeds its source".into(),
                    ))
                }
            })
            .transpose()
    }
}
