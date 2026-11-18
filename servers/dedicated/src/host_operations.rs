use super::*;
use crate::{metrics::HostSnapshot, operations::Operations};
impl Host {
    pub fn enable_diagnostics(&mut self, enabled: bool) {
        self.observing = enabled;
        if enabled {
            self.metrics = Default::default();
        }
        match &mut self.world {
            World::Demo(game) => game.enable_diagnostics(enabled),
            World::Saved(session) => session.enable_diagnostics(enabled),
        }
    }
    pub fn set_player_limit(&mut self, limit: usize) -> Result<(), SaveError> {
        if !(1..=64).contains(&limit) {
            return Err(failure("Player limit must be 1..64"));
        }
        self.player_limit = limit;
        Ok(())
    }
    pub fn configure_operations(&mut self, operations: Operations) -> Result<(), SaveError> {
        operations.validate().map_err(failure)?;
        self.enable_diagnostics(true);
        self.operations = Some(operations);
        Ok(())
    }
    pub fn diagnostics(&mut self) -> HostSnapshot {
        let mut active = std::collections::HashSet::new();
        for (_, state) in self.world.game().server.player_connections() {
            if let Some(control) = state.control() {
                active.insert(control.region_id);
            }
        }
        let (mut entities, mut stashed, mut items) = (0, 0, 0);
        for instance in &self.world.game().server.instances {
            if let Some(context) = instance.lock().unwrap().context() {
                let ctx = context.lock().unwrap();
                entities += ctx.map.entities.len();
                stashed += ctx.stashed_entities.len();
                items += ctx.map.items.len();
            }
        }
        let loaded = self.world.game().server.instances.len();
        let runtime = self.world.game().diagnostics();
        let saves = match &self.world {
            World::Saved(s) => s.diagnostics(),
            _ => None,
        };
        let healthy = !matches!(&self.world,World::Saved(s) if s.needs_reload());
        let scheduling = self.world.game().server.region_schedule();
        let inactive_region_policy = if self.world.game().server.has_region_scheduling() {
            "opt_in_pause_unoccupied; local_time_frozen; no_catchup"
        } else {
            "all_loaded_regions_tick; no_offline_catchup"
        };
        let m = &self.metrics;
        HostSnapshot {
            schema: 1,
            protocol: PROTOCOL_VERSION,
            generated_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u64::MAX as u128) as u64,
            uptime_ms: self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            mode: self.world.mode(),
            healthy,
            inactive_region_policy,
            scheduling,
            players: self
                .peers
                .values()
                .filter(|p| p.connection.is_some())
                .count(),
            player_limit: self.player_limit,
            peers: self.peers.len(),
            loaded_regions: loaded,
            regions_without_players: loaded.saturating_sub(active.len()),
            entities,
            stashed_entities: stashed,
            items,
            content_cache_bytes: self.content_bytes,
            socket_packets_queued: self
                .peers
                .values()
                .map(|p| p.output.max_capacity() - p.output.capacity())
                .sum(),
            peak_socket_queue: m.packet_queue_peak,
            messages_received: m.messages_in,
            packets_enqueued: m.packets_out,
            payload_bytes_enqueued: m.bytes_out,
            slow_disconnects: m.slow_disconnects,
            replica_disconnects: m.replica_disconnects,
            busy_rejections: m.busy_rejections,
            rate_rejections: m.rate_rejections,
            content_misses: m.content_misses,
            backups: m.backups,
            backup_failures: m.backup_failures,
            metrics_write_failures: m.metrics_write_failures,
            wall_time_clamped_us: m.wall_time_clamped_us,
            ticks_over_budget: m.ticks_over_budget,
            encoding: m.encoding.summary(),
            host_tick: m.tick.summary(),
            command: m.command.summary(),
            action_group: m.action_group.summary(),
            maintenance: m.maintenance.summary(),
            backup: m.backup.summary(),
            runtime,
            saves,
            transport: self.transport_metrics.as_ref().map(|m| m.snapshot()),
        }
    }
    pub fn write_metrics_now(&mut self) {
        if let Some(path) = self
            .operations
            .as_ref()
            .and_then(|o| o.metrics_file.clone())
        {
            let snapshot = self.diagnostics();
            if let Err(error) = crate::operations::write_metrics(&path, &snapshot) {
                self.metrics.metrics_write_failures =
                    self.metrics.metrics_write_failures.saturating_add(1);
                eprintln!("Eldiron metrics output failed: {error}");
            }
        }
    }
    pub async fn backup_to(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), SaveError> {
        let started = Instant::now();
        let result = match &mut self.world {
            World::Demo(_) => Err(failure("Disposable demos have no durable backup")),
            World::Saved(session) => match session.checkpoint().await {
                Ok(_) => eldiron_runtime::persistence::write_snapshot_new(path, session.saved()),
                Err(e) => Err(e),
            },
        };
        self.metrics.backup.record(started.elapsed());
        if result.is_ok() {
            self.metrics.backups = self.metrics.backups.saturating_add(1);
        } else {
            self.metrics.backup_failures = self.metrics.backup_failures.saturating_add(1);
        }
        result
    }
    pub(super) async fn housekeeping(&mut self) -> Result<(), SaveError> {
        let Some(mut operations) = self.operations.take() else {
            return Ok(());
        };
        let started = Instant::now();
        if operations.last_backup.elapsed() >= operations.backup_interval {
            if let World::Saved(session) = &self.world {
                let path = operations.backup_path(session.saved());
                let path = match path {
                    Ok(path) => path,
                    Err(error) => {
                        self.metrics.backup_failures =
                            self.metrics.backup_failures.saturating_add(1);
                        eprintln!("Eldiron backup preflight failed: {error}");
                        None
                    }
                };
                if let Some(path) = path {
                    if let Err(error) = self.backup_to(path).await {
                        if matches!(&self.world,World::Saved(s) if s.needs_reload()) {
                            self.operations = Some(operations);
                            return Err(error);
                        }
                        eprintln!("Eldiron backup failed: {error}");
                    } else if let World::Saved(session) = &self.world
                        && let Err(error) = operations.prune(session.saved())
                    {
                        self.metrics.backup_failures =
                            self.metrics.backup_failures.saturating_add(1);
                        eprintln!("Eldiron backup retention failed: {error}");
                    }
                }
            }
            operations.last_backup = Instant::now();
        }
        if operations.last_metrics.elapsed() >= Duration::from_secs(5) {
            if let Some(path) = &operations.metrics_file {
                let snapshot = self.diagnostics();
                if let Err(error) = crate::operations::write_metrics(path, &snapshot) {
                    self.metrics.metrics_write_failures =
                        self.metrics.metrics_write_failures.saturating_add(1);
                    eprintln!("Eldiron metrics output failed: {error}");
                }
            }
            operations.last_metrics = Instant::now();
        }
        self.metrics.maintenance.record(started.elapsed());
        self.operations = Some(operations);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn scheduled_backups_publish_coherent_heads_retain_latest_and_export_private_metrics() {
        let directory = std::env::temp_dir().join(format!("eldiron-periodic-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let project = crate::recovery::load_project(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../test_projects/3DStarter.eldiron"),
        )
        .unwrap();
        let mut host = Host::open(project, Some(directory.join("world.save")))
            .await
            .unwrap();
        let backup_dir = directory.join("backups");
        let metrics = directory.join("metrics.json");
        let mut operations = Operations::default();
        operations.backup_dir = Some(backup_dir.clone());
        operations.metrics_file = Some(metrics.clone());
        operations.backup_retain = 1;
        host.configure_operations(operations).unwrap();
        let mut previous_revision = 0;
        for count in 1..=2 {
            let operations = host.operations.as_mut().unwrap();
            operations.last_backup = Instant::now() - Duration::from_secs(301);
            operations.last_metrics = Instant::now() - Duration::from_secs(6);
            host.tick(Duration::from_millis(20)).await.unwrap();
            let files: Vec<_> = std::fs::read_dir(&backup_dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            assert_eq!(
                files.len(),
                1,
                "immutable backups must not leak owner lock files"
            );
            let image = eldiron_runtime::persistence::read_snapshot(&files[0]).unwrap();
            let World::Saved(session) = &host.world else {
                panic!()
            };
            assert_eq!(image.world, session.saved().world);
            assert_eq!(image.revision, session.saved().revision);
            assert!(image.revision > previous_revision);
            previous_revision = image.revision;
            let exported: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&metrics).unwrap()).unwrap();
            assert_eq!(exported["backups"], count);
            assert_eq!(exported["backup_failures"], 0);
            assert_eq!(exported["healthy"], true);
        }
        host.shutdown().await.unwrap();
        drop(host);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
