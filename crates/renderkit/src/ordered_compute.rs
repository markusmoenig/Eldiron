//! Ordered compute work with bounded unsubmitted backend command buffers.
use crate::{GpuContext, wgpu};

// Native backends may allocate several raw command buffers per compute pass.
// Submit regularly without waiting, retaining every inter-pass storage barrier.
const PASSES_PER_SUBMISSION: usize = 128;

pub struct OrderedCompute<'a> {
    context: &'a GpuContext,
    label: &'static str,
    encoder: Option<wgpu::CommandEncoder>,
    passes: usize,
    submissions: usize,
}
impl<'a> OrderedCompute<'a> {
    pub fn new(context: &'a GpuContext, label: &'static str) -> Self {
        Self {
            context,
            label,
            encoder: None,
            passes: 0,
            submissions: 0,
        }
    }
    fn submit(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            self.context.queue.submit([encoder.finish()]);
            self.submissions += 1;
        }
        self.passes = 0;
    }
    /// Start the next ordered pass. Previous batches are submitted on this same
    /// queue; this never polls, waits, or merges storage-visible pass boundaries.
    pub fn begin_pass(&mut self) -> wgpu::ComputePass<'_> {
        if self.passes == PASSES_PER_SUBMISSION {
            self.submit();
        }
        let encoder = self.encoder.get_or_insert_with(|| {
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some(self.label),
                })
        });
        self.passes += 1;
        encoder.begin_compute_pass(&Default::default())
    }
    /// Submit the final batch. Dropping instead abandons only the remaining
    /// unsubmitted batch, as with a normal command encoder.
    pub fn finish(mut self) -> usize {
        self.submit();
        self.submissions
    }
}
