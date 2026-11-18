use super::incremental::ImageIndex;
use super::records::invalid;
use super::*;
use crate::{GameSession, PlayerConnectionId, Project, SessionOptions};
use std::collections::BTreeMap;
use theframework_core::Uuid;

#[derive(Default)]
struct PendingGroup {
    journals: BTreeMap<Uuid, ActionJournal>,
    operations: usize,
}
enum PreparedCommit {
    Snapshot(PreparedSave),
    Update(PreparedUpdate),
}
impl PreparedCommit {
    fn image(&self) -> &WorldSave {
        match self {
            Self::Snapshot(s) => s.image(),
            Self::Update(s) => s.image(),
        }
    }
}

/// Opt-in persistence owner. Default GameSession/Creator Play remain temporary.
/// Storage awaits borrow this owner exclusively. On ambiguous failure reopen
/// before more writes. Drop cannot silently save: hosts call shutdown instead.
pub struct DurableSession<S: WorldStore> {
    pub game: GameSession,
    diagnostics: Option<crate::diagnostics::SaveTimings>,
    store: S,
    head: WorldSave,
    poisoned: bool,
    checkpoint_interval: std::time::Duration,
    since_checkpoint: std::time::Duration,
    incarnation: Uuid,
    index: Option<ImageIndex>,
    pending: Option<PendingGroup>,
}
impl<S: WorldStore> DurableSession<S> {
    pub async fn open(
        project: Project,
        mut store: S,
        purpose: SavePurpose,
        options: SessionOptions,
    ) -> Result<Self, SaveError> {
        let content = content_revision(&project)?;
        let loaded = store.load().await?;
        if let Some(saved) = &loaded {
            saved.validate()?;
            if saved.purpose != purpose {
                return Err(SaveError::Incompatible(
                    "Save purpose/test slot differs; select an isolated save".into(),
                ));
            }
            if saved.content != content {
                return Err(SaveError::Incompatible("Project content changed; use matching .eldiron content or an explicit migration".into()));
            }
        }
        let mut game = GameSession::new(project);
        game.start_with_state(options, loaded.as_ref().map(|s| &s.world_state))
            .map_err(invalid)?;
        if let Some(saved) = &loaded {
            game.clock =
                rusterix::simulation::SimulationClock::restore(&saved.clock).map_err(invalid)?;
            game.pump.restore_counters(saved.tick, saved.movement_tick);
            if let Some(id) = saved.local_character {
                let character = saved.characters.iter().find(|c| c.id == id).unwrap();
                game.server
                    .attach_saved_character(
                        game.server.local_connection(),
                        character.region,
                        character.entity,
                    )
                    .map_err(invalid)?;
                game.local_player_started = true;
                game.current_map = game
                    .project
                    .regions
                    .iter()
                    .find(|r| r.map.id == character.region)
                    .ok_or_else(|| invalid("Local region missing"))?
                    .map
                    .name
                    .clone();
            }
            game.refresh_replication().map_err(invalid)?;
        }
        settle(&mut game)?;
        let is_new = loaded.is_none();
        let head = match loaded {
            Some(saved) => saved,
            None => WorldSave {
                version: SAVE_VERSION,
                world: Uuid::new_v4(),
                purpose,
                content,
                revision: 0,
                characters: vec![],
                local_character: None,
                requests: Default::default(),
                host_data: Default::default(),
                action_streams: Default::default(),
                world_state: game.server.capture_durable_world().map_err(invalid)?,
                clock: game.clock.save(),
                tick: game.tick_sequence(),
                movement_tick: game.movement_tick_sequence(),
            },
        };
        let incarnation = game.driver().session_id();
        let mut session = Self {
            game,
            diagnostics: None,
            store,
            head,
            poisoned: false,
            checkpoint_interval: std::time::Duration::from_secs(5),
            since_checkpoint: std::time::Duration::ZERO,
            incarnation,
            index: None,
            pending: None,
        };
        if is_new {
            session.checkpoint().await?;
        }
        Ok(session)
    }
    pub fn enable_diagnostics(&mut self, enabled: bool) {
        self.game.enable_diagnostics(enabled);
        self.diagnostics = enabled.then(Default::default);
    }
    pub fn diagnostics(&self) -> Option<crate::diagnostics::SaveDiagnostics> {
        let d = self.diagnostics.as_ref()?;
        Some(crate::diagnostics::SaveDiagnostics {
            encoding: d.encoding.summary(),
            capture: d.capture.summary(),
            commit: d.commit.summary(),
            failures: d.failures,
            poisoned: self.poisoned,
            revision: self.head.revision,
            storage: self.store.diagnostics(),
            groups: d.groups,
            grouped_actions: d.grouped_actions,
        })
    }
    async fn commit_image(
        &mut self,
        expected: Option<u64>,
        saved: &PreparedCommit,
    ) -> Result<(), SaveError> {
        let started = self.diagnostics.as_ref().map(|_| instant::Instant::now());
        let result = match saved {
            PreparedCommit::Snapshot(saved) => self.store.commit_prepared(expected, saved).await,
            PreparedCommit::Update(saved) => {
                self.store
                    .commit_update(expected.ok_or(SaveError::Conflict)?, saved)
                    .await
            }
        };
        if let (Some(started), Some(d)) = (started, &mut self.diagnostics) {
            d.commit.record(started.elapsed());
            d.failures = d.failures.saturating_add(u64::from(result.is_err()));
        }
        result
    }
    fn prepare_image(
        &mut self,
        saved: WorldSave,
        checkpoint: bool,
    ) -> Result<PreparedCommit, SaveError> {
        let started = self.diagnostics.as_ref().map(|_| instant::Instant::now());
        let result = (|| {
            if !checkpoint && self.store.supports_incremental() && self.head.revision > 0 {
                if self.index.is_none() {
                    self.index = Some(ImageIndex::from_image(&self.head)?);
                }
                Ok(PreparedCommit::Update(PreparedUpdate::new(
                    &self.head,
                    self.index.as_ref().unwrap(),
                    saved,
                )?))
            } else {
                Ok(PreparedCommit::Snapshot(PreparedSave::new(&saved)?))
            }
        })();
        if let (Some(started), Some(d)) = (started, &mut self.diagnostics) {
            d.encoding.record(started.elapsed());
        }
        result
    }
    fn install(&mut self, saved: PreparedCommit) {
        match saved {
            PreparedCommit::Snapshot(saved) => {
                self.head = saved.into_image();
                self.index = None;
            }
            PreparedCommit::Update(saved) => {
                let (image, index) = saved.into_head();
                self.head = image;
                self.index = Some(index);
            }
        }
        self.since_checkpoint = std::time::Duration::ZERO;
    }
    fn outside_group(&self) -> Result<(), SaveError> {
        if self.pending.is_some() {
            return Err(SaveError::Busy);
        }
        Ok(())
    }
    /// A host must buffer all replies and frames until `finish_sequenced_group`
    /// succeeds. Only sequenced actions may run inside this exclusive group.
    pub fn begin_sequenced_group(&mut self) -> Result<(), SaveError> {
        if self.needs_reload() {
            return Err(SaveError::NeedsReload);
        }
        self.outside_group()?;
        self.pending = Some(PendingGroup::default());
        Ok(())
    }
    /// Committed metadata remains visible through `saved`; only this accessor
    /// includes staged receipts needed to sequence subsequent actions.
    pub fn action_journal(&self, stream: Uuid) -> Option<&ActionJournal> {
        self.pending
            .as_ref()
            .and_then(|g| g.journals.get(&stream))
            .or_else(|| self.head.action_streams.get(&stream))
    }
    pub fn abort_sequenced_group(&mut self) {
        if self.pending.take().is_some() {
            self.poisoned = true;
        }
    }
    pub async fn finish_sequenced_group(&mut self) -> Result<u64, SaveError> {
        let group = self
            .pending
            .take()
            .ok_or_else(|| invalid("No pending action group"))?;
        if self.needs_reload() {
            return Err(SaveError::NeedsReload);
        }
        if group.operations == 0 {
            return Ok(self.head.revision);
        }
        let prepared = (|| {
            let mut saved = self.capture()?;
            saved.action_streams.extend(group.journals);
            self.prepare_image(saved, false)
        })();
        let saved = match prepared {
            Ok(s) => s,
            Err(e) => {
                self.poisoned = true;
                return Err(e);
            }
        };
        if let Err(e) = self.commit_image(Some(self.head.revision), &saved).await {
            self.poisoned = true;
            return Err(e);
        }
        self.install(saved);
        if let Some(d) = &mut self.diagnostics {
            d.groups += 1;
            d.grouped_actions += group.operations as u64;
        }
        Ok(self.head.revision)
    }
    pub fn needs_reload(&self) -> bool {
        self.poisoned || self.game.driver().session_id() != self.incarnation
    }
    pub fn saved(&self) -> &WorldSave {
        &self.head
    }
    pub fn store(&self) -> &S {
        &self.store
    }
    /// Periodic checkpoints bound crash loss for ordinary simulation changes.
    /// Economic operations use commit_operation for immediate durability.
    pub fn set_checkpoint_interval(&mut self, interval: std::time::Duration) {
        self.checkpoint_interval = interval.max(std::time::Duration::from_millis(1));
    }
    pub async fn advance(
        &mut self,
        elapsed: std::time::Duration,
    ) -> Result<crate::SessionUpdate, SaveError> {
        self.outside_group()?;
        if self.poisoned || self.game.driver().session_id() != self.incarnation {
            return Err(SaveError::NeedsReload);
        }
        let update = self.game.advance(elapsed);
        if let Some(error) = &update.replication_error {
            return Err(invalid(error));
        }
        self.since_checkpoint = self.since_checkpoint.saturating_add(elapsed);
        if self.since_checkpoint >= self.checkpoint_interval {
            self.checkpoint().await?;
        }
        Ok(update)
    }
    fn capture(&mut self) -> Result<WorldSave, SaveError> {
        let started = self.diagnostics.as_ref().map(|_| instant::Instant::now());
        let result = self.capture_inner();
        if let (Some(started), Some(d)) = (started, &mut self.diagnostics) {
            d.capture.record(started.elapsed());
        }
        result
    }
    fn capture_inner(&mut self) -> Result<WorldSave, SaveError> {
        if self.poisoned || self.game.driver().session_id() != self.incarnation {
            return Err(SaveError::NeedsReload);
        }
        if !self.game.is_started() || self.game.server.state == rusterix::ServerState::Off {
            return Err(invalid("Cannot checkpoint a stopped world"));
        }
        settle(&mut self.game)?;
        let mut saved = WorldSave {
            version: self.head.version,
            world: self.head.world,
            purpose: self.head.purpose.clone(),
            content: self.head.content.clone(),
            revision: self
                .head
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("Save revision exhausted"))?,
            characters: self.head.characters.clone(),
            local_character: self.head.local_character,
            requests: self.head.requests.clone(),
            host_data: self.head.host_data.clone(),
            action_streams: self.head.action_streams.clone(),
            world_state: self.game.server.capture_durable_world().map_err(invalid)?,
            clock: self.game.clock.save(),
            tick: self.game.tick_sequence(),
            movement_tick: self.game.movement_tick_sequence(),
        };
        if saved.world_state.next_id < self.head.world_state.next_id
            || saved.world_state.regions.len() != self.head.world_state.regions.len()
            || saved.world_state.regions.iter().any(|region| {
                !self
                    .head
                    .world_state
                    .regions
                    .iter()
                    .any(|old| old.geometry.id == region.geometry.id)
            })
        {
            return Err(invalid(
                "World was replaced; reopen a durable session for the selected save",
            ));
        }
        saved.clock = self.game.clock.save();
        saved.tick = self.game.tick_sequence();
        saved.movement_tick = self.game.movement_tick_sequence();
        let local = self
            .game
            .server
            .player_control(self.game.server.local_connection())
            .map(|c| c.entity_id);
        let mut characters = vec![];
        for region in &saved.world_state.regions {
            for entity in region
                .entities
                .iter()
                .chain(&region.stashed)
                .filter(|e| e.attributes.get_bool_default("player", false))
            {
                let mut character = saved
                    .characters
                    .iter()
                    .find(|c| c.entity == entity.id)
                    .cloned()
                    .unwrap_or(CharacterRecord {
                        id: Uuid::new_v4(),
                        account: None,
                        entity: entity.id,
                        region: region.geometry.id,
                    });
                character.region = region.geometry.id;
                if local == Some(entity.id) {
                    saved.local_character = Some(character.id);
                }
                characters.push(character);
            }
        }
        saved.characters = characters;
        if saved
            .local_character
            .is_some_and(|id| !saved.characters.iter().any(|c| c.id == id))
        {
            saved.local_character = None;
        }
        saved.validate()?;
        Ok(saved)
    }
    pub async fn checkpoint(&mut self) -> Result<u64, SaveError> {
        self.outside_group()?;
        let saved = self.capture()?;
        let saved = self
            .prepare_image(saved, true)
            .inspect_err(|_| self.poisoned = true)?;
        let expected = (self.head.revision > 0).then_some(self.head.revision);
        if let Err(error) = self.commit_image(expected, &saved).await {
            self.poisoned = true;
            return Err(error);
        }
        self.install(saved);
        Ok(self.head.revision)
    }
    /// Commit private host metadata without consuming the character-creation
    /// receipt ledger. Storage publication precedes in-memory authorization.
    pub async fn commit_host_metadata(
        &mut self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<u64, SaveError> {
        self.outside_group()?;
        if key.is_empty() || key.len() > 64 {
            return Err(invalid("Invalid private metadata key"));
        }
        let mut saved = self.capture()?;
        saved.host_data.insert(key.into(), value);
        saved.validate()?;
        let prepared = self
            .prepare_image(saved, false)
            .inspect_err(|_| self.poisoned = true)?;
        if let Err(error) = self.commit_image(Some(self.head.revision), &prepared).await {
            self.poisoned = true;
            return Err(error);
        }
        self.install(prepared);
        Ok(self.head.revision)
    }
    /// Edit a staged record, never live simulation. Fingerprints identify exact
    /// participants and quantities. A committed retry returns its saved outcome.
    pub async fn commit_operation<F>(
        &mut self,
        request: Uuid,
        fingerprint: &str,
        edit: F,
    ) -> Result<serde_json::Value, SaveError>
    where
        F: FnOnce(&mut WorldSave) -> Result<serde_json::Value, SaveError>,
    {
        self.outside_group()?;
        if self.poisoned || self.game.driver().session_id() != self.incarnation {
            return Err(SaveError::NeedsReload);
        }
        if let Some(committed) = self.head.requests.get(&request) {
            return if committed.fingerprint == fingerprint {
                Ok(committed.outcome.clone())
            } else {
                Err(SaveError::Conflict)
            };
        }
        if request.is_nil()
            || fingerprint.is_empty()
            || fingerprint.len() > 1024
            || self.head.requests.len() == MAX_DURABLE_REQUESTS
        {
            return Err(invalid("Invalid request or durable request ledger full"));
        }
        let mut saved = self.capture()?;
        let clock = saved.clock.clone();
        let counters = (saved.tick, saved.movement_tick);
        let outcome = edit(&mut saved)?;
        if saved.world != self.head.world
            || saved.content != self.head.content
            || saved.purpose != self.head.purpose
            || saved.revision != self.head.revision + 1
            || saved.requests != self.head.requests
            || saved.action_streams != self.head.action_streams
            || saved.clock != clock
            || (saved.tick, saved.movement_tick) != counters
        {
            return Err(invalid("Operation changed save identity/revision"));
        }
        saved.requests.insert(
            request,
            CommittedRequest {
                fingerprint: fingerprint.into(),
                outcome: outcome.clone(),
            },
        );
        // Roundtrip validation prevents non-finite data silently becoming JSON null.
        let saved = self.prepare_image(saved, false)?;
        let prepared = self
            .game
            .server
            .prepare_durable_world(&saved.image().world_state)
            .map_err(invalid)?;
        if let Err(error) = self.commit_image(Some(self.head.revision), &saved).await {
            self.poisoned = true;
            return Err(error);
        }
        self.game.server.apply_durable_world(prepared);
        self.install(saved);
        if let Some(name) = self
            .game
            .server
            .player_region_name(self.game.server.local_connection())
        {
            self.game.current_map = name.to_owned();
        }
        for (connection, _) in self.game.server.player_connections() {
            self.game
                .pump
                .request_baseline(connection)
                .map_err(invalid)?;
        }
        self.game.refresh_replication().map_err(invalid)?;
        Ok(outcome)
    }
    /// Single-owner host transaction for operations requiring live startup/VM
    /// callbacks. `edit` may associate host/character metadata but cannot rewrite
    /// the captured live world. No frame or reply may escape until success.
    /// Any failure after `apply` begins poisons the owner: discard it and reopen
    /// the last atomic checkpoint. This fail-closed contract avoids publishing
    /// grants/characters which storage did not commit. Retries return the receipt
    /// before running callbacks, including after restart.
    pub async fn commit_live_operation<A, E>(
        &mut self,
        request: Uuid,
        fingerprint: &str,
        apply: A,
        edit: E,
    ) -> Result<serde_json::Value, SaveError>
    where
        A: FnOnce(&mut GameSession) -> Result<serde_json::Value, SaveError>,
        E: FnOnce(&mut WorldSave, serde_json::Value) -> Result<serde_json::Value, SaveError>,
    {
        self.outside_group()?;
        if self.poisoned || self.game.driver().session_id() != self.incarnation {
            return Err(SaveError::NeedsReload);
        }
        if let Some(committed) = self.head.requests.get(&request) {
            return if committed.fingerprint == fingerprint {
                Ok(committed.outcome.clone())
            } else {
                Err(SaveError::Conflict)
            };
        }
        if request.is_nil()
            || fingerprint.is_empty()
            || fingerprint.len() > 1024
            || self.head.requests.len() == MAX_DURABLE_REQUESTS
        {
            return Err(invalid("Invalid request or durable request ledger full"));
        }
        self.commit_live_inner(
            LiveReceipt::Request(request, fingerprint.to_owned()),
            apply,
            edit,
        )
        .await
    }
    /// Character sequence checks and world effects commit in the same atomic
    /// save. Old receipts may be compacted; the high-water mark never decreases.
    pub async fn commit_sequenced_live<A>(
        &mut self,
        stream: Uuid,
        sequence: u64,
        fingerprint: &str,
        apply: A,
    ) -> Result<serde_json::Value, SaveError>
    where
        A: FnOnce(&mut GameSession) -> Result<serde_json::Value, SaveError>,
    {
        if self.poisoned || self.game.driver().session_id() != self.incarnation {
            return Err(SaveError::NeedsReload);
        }
        if !self.head.characters.iter().any(|c| c.id == stream) {
            return Err(invalid("Unknown character action stream"));
        }
        if let Some(outcome) = self
            .action_journal(stream)
            .unwrap_or(&ActionJournal::default())
            .check(sequence, fingerprint)?
        {
            return Ok(outcome);
        }
        if self.pending.is_some() {
            if self.pending.as_ref().unwrap().operations >= MAX_GROUP_ACTIONS {
                return Err(SaveError::Busy);
            }
            let value = (|| {
                settle(&mut self.game)?;
                apply(&mut self.game)
            })();
            let value = match value {
                Ok(value) => value,
                Err(e) => {
                    self.poisoned = true;
                    return Err(e);
                }
            };
            let group = self.pending.as_mut().unwrap();
            let journal = group.journals.entry(stream).or_insert_with(|| {
                self.head
                    .action_streams
                    .get(&stream)
                    .cloned()
                    .unwrap_or_default()
            });
            if let Err(e) = journal.append(sequence, fingerprint.to_owned(), value.clone()) {
                self.poisoned = true;
                return Err(e);
            }
            group.operations += 1;
            return Ok(value);
        }
        self.commit_live_inner(
            LiveReceipt::Action(stream, sequence, fingerprint.to_owned()),
            apply,
            |_, value| Ok(value),
        )
        .await
    }
    async fn commit_live_inner<A, E>(
        &mut self,
        receipt: LiveReceipt,
        apply: A,
        edit: E,
    ) -> Result<serde_json::Value, SaveError>
    where
        A: FnOnce(&mut GameSession) -> Result<serde_json::Value, SaveError>,
        E: FnOnce(&mut WorldSave, serde_json::Value) -> Result<serde_json::Value, SaveError>,
    {
        settle(&mut self.game)?;
        let staged = (|| {
            let value = apply(&mut self.game)?;
            let mut saved = self.capture()?;
            let clock = saved.clock.clone();
            let captured_world = if matches!(receipt, LiveReceipt::Request(..)) {
                Some(serde_json::to_value(&saved.world_state).map_err(invalid)?)
            } else {
                None
            };
            let counters = (saved.tick, saved.movement_tick);
            let outcome = edit(&mut saved, value)?;
            if saved.world != self.head.world
                || saved.content != self.head.content
                || saved.purpose != self.head.purpose
                || saved.revision != self.head.revision + 1
                || saved.requests != self.head.requests
                || saved.action_streams != self.head.action_streams
                || captured_world.as_ref().is_some_and(|world| {
                    serde_json::to_value(&saved.world_state)
                        .as_ref()
                        .map_or(true, |value| value != world)
                })
                || saved.clock != clock
                || (saved.tick, saved.movement_tick) != counters
            {
                return Err(invalid("Operation changed save identity/revision"));
            }
            match &receipt {
                LiveReceipt::Request(request, fingerprint) => {
                    saved.requests.insert(
                        *request,
                        CommittedRequest {
                            fingerprint: fingerprint.clone(),
                            outcome: outcome.clone(),
                        },
                    );
                }
                LiveReceipt::Action(stream, sequence, fingerprint) => {
                    saved.action_streams.entry(*stream).or_default().append(
                        *sequence,
                        fingerprint.clone(),
                        outcome.clone(),
                    )?;
                }
            }
            let saved = self.prepare_image(saved, false)?;
            // Validate all engine invariants before publishing a checkpoint.
            // A live action's world was captured from this exact running engine
            // and `edit` cannot rewrite it. Roundtrip validation checks identities
            // without rebuilding all collision/VM programs on every combat click.
            if matches!(receipt, LiveReceipt::Request(..)) {
                self.game
                    .server
                    .prepare_durable_world(&saved.image().world_state)
                    .map_err(invalid)?;
            }
            Ok((saved, outcome))
        })();
        let (saved, outcome) = match staged {
            Ok(staged) => staged,
            Err(error) => {
                self.poisoned = true;
                return Err(error);
            }
        };
        if let Err(error) = self.commit_image(Some(self.head.revision), &saved).await {
            self.poisoned = true;
            return Err(error);
        }
        self.install(saved);
        Ok(outcome)
    }
    /// Trusted host selection. Authenticated account checks belong to the host.
    pub fn join_character(
        &mut self,
        connection: PlayerConnectionId,
        character: Uuid,
    ) -> Result<(), SaveError> {
        self.outside_group()?;
        if self.poisoned || self.game.driver().session_id() != self.incarnation {
            return Err(SaveError::NeedsReload);
        }
        settle(&mut self.game)?;
        let record = self
            .head
            .characters
            .iter()
            .find(|c| c.id == character)
            .ok_or_else(|| invalid("Unknown durable character"))?;
        let region = self
            .game
            .server
            .instances
            .iter()
            .find_map(|i| {
                let context = i.lock().ok()?.context()?;
                let ctx = context.lock().ok()?;
                ctx.find_entity(record.entity).map(|_| ctx.map.id)
            })
            .ok_or_else(|| invalid("Saved character no longer exists"))?;
        self.game
            .server
            .attach_saved_character(connection, region, record.entity)
            .map_err(invalid)?;
        if connection == self.game.server.local_connection() {
            self.game.local_player_started = true;
            if let Some(name) = self.game.server.player_region_name(connection) {
                self.game.current_map = name.to_owned();
            }
        }
        self.game.refresh_replication().map_err(invalid)
    }
    pub async fn shutdown(&mut self) -> Result<(), SaveError> {
        if !self.game.is_started() {
            return Ok(());
        }
        self.checkpoint().await?;
        self.game.stop();
        Ok(())
    }
}
fn settle(game: &mut GameSession) -> Result<(), SaveError> {
    for _ in 0..32 {
        if let Some(error) = game.poll().replication_error {
            return Err(invalid(error));
        }
        if game.server.persistence_ready() {
            return Ok(());
        }
    }
    Err(SaveError::Busy)
}

enum LiveReceipt {
    Request(Uuid, String),
    Action(Uuid, u64, String),
}
