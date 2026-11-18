use rusterix::diagnostics::{Timing, TimingSummary};
use serde::Serialize;
use std::{
    sync::Mutex,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
pub struct TransportMetrics {
    pub accepted: AtomicU64,
    pub refused: AtomicU64,
    pub active: AtomicU64,
    pub websocket_rx_bytes: AtomicU64,
    pub websocket_tx_bytes: AtomicU64,
    pub http_tx_bytes: AtomicU64,
    pub command_queue_peak: AtomicU64,
    pub hash_busy: AtomicU64,
    pub malformed: AtomicU64,
    pub write: Mutex<Timing>,
}
impl TransportMetrics {
    pub fn snapshot(&self) -> TransportSnapshot {
        let read = |v: &AtomicU64| v.load(Ordering::Relaxed);
        TransportSnapshot {
            accepted: read(&self.accepted),
            refused: read(&self.refused),
            active: read(&self.active),
            websocket_payload_bytes_received: read(&self.websocket_rx_bytes),
            websocket_payload_bytes_sent: read(&self.websocket_tx_bytes),
            http_payload_bytes_sent: read(&self.http_tx_bytes),
            command_queue_peak: read(&self.command_queue_peak),
            hash_busy: read(&self.hash_busy),
            malformed: read(&self.malformed),
            socket_write: self.write.lock().unwrap().summary(),
        }
    }
}
#[derive(Serialize)]
pub struct TransportSnapshot {
    pub accepted: u64,
    pub refused: u64,
    pub active: u64,
    pub websocket_payload_bytes_received: u64,
    pub websocket_payload_bytes_sent: u64,
    pub http_payload_bytes_sent: u64,
    pub command_queue_peak: u64,
    pub hash_busy: u64,
    pub malformed: u64,
    pub socket_write: TimingSummary,
}
#[derive(Default)]
pub(crate) struct HostMetrics {
    pub encoding: Timing,
    pub tick: Timing,
    pub command: Timing,
    pub action_group: Timing,
    pub maintenance: Timing,
    pub backup: Timing,
    pub messages_in: u64,
    pub packets_out: u64,
    pub bytes_out: u64,
    pub packet_queue_peak: usize,
    pub slow_disconnects: u64,
    pub replica_disconnects: u64,
    pub busy_rejections: u64,
    pub rate_rejections: u64,
    pub content_misses: u64,
    pub backups: u64,
    pub backup_failures: u64,
    pub metrics_write_failures: u64,
    pub wall_time_clamped_us: u64,
    pub ticks_over_budget: u64,
}
#[derive(Serialize)]
pub struct HostSnapshot {
    pub schema: u32,
    pub protocol: u16,
    pub generated_unix_ms: u64,
    pub uptime_ms: u64,
    pub mode: eldiron_network::HostMode,
    pub healthy: bool,
    pub inactive_region_policy: &'static str,
    pub scheduling: Vec<rusterix::server::scheduling::RegionScheduleSummary>,
    pub players: usize,
    pub player_limit: usize,
    pub peers: usize,
    pub loaded_regions: usize,
    pub regions_without_players: usize,
    pub entities: usize,
    pub stashed_entities: usize,
    pub items: usize,
    pub content_cache_bytes: usize,
    pub socket_packets_queued: usize,
    pub peak_socket_queue: usize,
    pub messages_received: u64,
    pub packets_enqueued: u64,
    pub payload_bytes_enqueued: u64,
    pub slow_disconnects: u64,
    pub replica_disconnects: u64,
    pub busy_rejections: u64,
    pub rate_rejections: u64,
    pub content_misses: u64,
    pub backups: u64,
    pub backup_failures: u64,
    pub metrics_write_failures: u64,
    pub wall_time_clamped_us: u64,
    pub ticks_over_budget: u64,
    pub encoding: TimingSummary,
    pub host_tick: TimingSummary,
    pub command: TimingSummary,
    pub action_group: TimingSummary,
    pub maintenance: TimingSummary,
    pub backup: TimingSummary,
    pub runtime: Option<eldiron_runtime::diagnostics::SessionDiagnostics>,
    pub saves: Option<eldiron_runtime::diagnostics::SaveDiagnostics>,
    pub transport: Option<TransportSnapshot>,
}
