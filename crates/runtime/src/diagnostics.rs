use rusterix::diagnostics::{RegionTimingSummary, Timing, TimingSummary};
use serde::Serialize;
#[derive(Default)]
pub(crate) struct PumpTimings {
    pub simulation: Timing,
    pub collection: Timing,
    pub replication: Timing,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ReplicationDiagnostics {
    pub streams: usize,
    pub subscribed_streams: usize,
    pub queued_frames: usize,
    pub peak_stream_frames: usize,
    pub overflowed_streams: usize,
    pub pending_events: usize,
    pub projected_entities: usize,
    pub projected_items: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct SessionDiagnostics {
    pub simulation: TimingSummary,
    pub collection: TimingSummary,
    pub replication: TimingSummary,
    pub regions: Vec<RegionTimingSummary>,
    pub queues: ReplicationDiagnostics,
}
#[derive(Default)]
pub(crate) struct SaveTimings {
    pub encoding: Timing,
    pub capture: Timing,
    pub commit: Timing,
    pub failures: u64,
    pub groups: u64,
    pub grouped_actions: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SaveDiagnostics {
    pub encoding: TimingSummary,
    pub capture: TimingSummary,
    pub commit: TimingSummary,
    pub failures: u64,
    pub groups: u64,
    pub grouped_actions: u64,
    pub poisoned: bool,
    pub revision: u64,
    pub storage: crate::persistence::StoreDiagnostics,
}
