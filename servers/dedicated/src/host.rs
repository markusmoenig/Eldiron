#[path = "account_security.rs"]
mod account_security;
#[path = "gameplay.rs"]
mod gameplay;
#[path = "host_operations.rs"]
mod operations_impl;
#[path = "scheduling.rs"]
mod scheduling;
#[path = "spatial.rs"]
mod spatial;
use crate::auth::{Account, Directory, SharedDirectory};
use eldiron_network::*;
use eldiron_runtime::{
    GameSession, PlayerConnectionId, PlayerStartOptions, Project, SessionOptions,
    persistence::{DurableSession, FileWorldStore, SaveError, SavePurpose},
};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use theframework_core::Uuid;
use tokio::sync::{mpsc, oneshot};
pub type PeerId = Uuid;
impl Command {
    pub fn is_action(&self) -> bool {
        matches!(
            self,
            Self::Message {
                message: ClientMessage::Action(_),
                ..
            }
        )
    }
}
pub enum Command {
    Open {
        peer: PeerId,
        output: mpsc::Sender<Vec<u8>>,
    },
    Message {
        peer: PeerId,
        message: ClientMessage,
    },
    Verified {
        peer: PeerId,
        account: Uuid,
        security_revision: u64,
    },
    Register {
        peer: PeerId,
        username: String,
        hash: String,
    },
    SecurityRejected {
        peer: PeerId,
        code: RejectCode,
    },
    PasswordContext {
        peer: PeerId,
        reply: oneshot::Sender<Result<Account, RejectCode>>,
    },
    PasswordVerified {
        peer: PeerId,
        account: Uuid,
        security_revision: u64,
        hash: String,
    },
    Close(PeerId),
    Content {
        key: String,
        reply: oneshot::Sender<Option<Arc<Vec<u8>>>>,
    },
}
struct Peer {
    output: mpsc::Sender<Vec<u8>>,
    hello: bool,
    account: Option<Uuid>,
    connection: Option<PlayerConnectionId>,
    stream: Option<Uuid>,
    journal: eldiron_runtime::persistence::ActionJournal,
    offers: HashMap<Uuid, OfferedChoice>,
    offer_context: Option<(Uuid, u64, Option<Uuid>)>,
    action_window: (Instant, u32),
    preloaded: HashMap<eldiron_runtime::replication::RegionContentRef, String>,
    sent_content: HashMap<eldiron_runtime::replication::RegionContentRef, String>,
    opened: Instant,
    queued_lengths: std::collections::VecDeque<usize>,
    last: Instant,
}
struct Token {
    account: Uuid,
    expires: Instant,
    security_revision: u64,
}
pub enum World {
    Demo(GameSession),
    Saved(DurableSession<FileWorldStore>),
}
impl World {
    pub fn game(&mut self) -> &mut GameSession {
        match self {
            Self::Demo(game) => game,
            Self::Saved(session) => &mut session.game,
        }
    }
    pub fn mode(&self) -> HostMode {
        match self {
            Self::Demo(_) => HostMode::Demo,
            Self::Saved(_) => HostMode::Persistent,
        }
    }
}
#[derive(Default)]
struct StagedOutput {
    packets: Vec<(PeerId, Vec<u8>)>,
    totals: HashMap<PeerId, (usize, usize)>,
    closed: std::collections::HashSet<PeerId>,
}
pub struct Host {
    staging: Option<StagedOutput>,
    scheduling: rusterix::server::scheduling::RegionSchedulingPolicy,
    started: Instant,
    observing: bool,
    metrics: crate::metrics::HostMetrics,
    operations: Option<crate::operations::Operations>,
    pub(crate) transport_metrics: Option<Arc<crate::metrics::TransportMetrics>>,
    player_limit: usize,
    pub world: World,
    project: Project,
    pub directory: SharedDirectory,
    peers: HashMap<PeerId, Peer>,
    tokens: HashMap<String, Token>,
    content: HashMap<String, Arc<Vec<u8>>>,
    content_bytes: usize,
    content_refs: HashMap<(eldiron_runtime::replication::RegionContentRef, u64), String>,
    spatial: eldiron_runtime::replication::SpatialSubscriptions,
    content_revision: String,
    demo_spawns: usize,
}
fn failure(value: impl std::fmt::Display) -> SaveError {
    SaveError::Invalid(value.to_string())
}
fn options() -> SessionOptions {
    SessionOptions {
        create_local_player: false,
        require_start_region: true,
        ..Default::default()
    }
}
fn name(value: &str) -> bool {
    (1..=32).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
        && value.trim() == value
}
fn spawn(
    game: &mut GameSession,
    connection: PlayerConnectionId,
    name: &str,
) -> Result<u32, SaveError> {
    let region = game.current_map.clone();
    game.start_connected_player(
        connection,
        &region,
        PlayerStartOptions {
            name: Some(name),
            ..Default::default()
        },
    )
    .map_err(failure)?;
    for _ in 0..32 {
        game.poll();
        if let Some(control) = game.server.player_control(connection)
            && game.server.persistence_ready()
        {
            return Ok(control.entity_id);
        }
    }
    Err(SaveError::Busy)
}
impl Host {
    pub async fn open(
        project: Project,
        state: Option<std::path::PathBuf>,
    ) -> Result<Self, SaveError> {
        let store = state.map(FileWorldStore::open_journaled).transpose()?;
        Self::with_store(project, store).await
    }
    /// Trusted host storage selection, also used to compare storage backends
    /// with identical content. Normal dedicated startup selects the journal.
    pub async fn with_store(
        project: Project,
        store: Option<FileWorldStore>,
    ) -> Result<Self, SaveError> {
        let spatial = spatial::configuration(&project)?;
        let scheduling = scheduling::configuration(&project)?;
        let content_revision =
            eldiron_network::project_content_revision(&project).map_err(failure)?;
        let (mut world, directory) = if let Some(store) = store {
            let session =
                DurableSession::open(project.clone(), store, SavePurpose::Dedicated, options())
                    .await?;
            let directory = match session.saved().host_data.get("accounts.v1") {
                Some(value) => serde_json::from_value(value.clone()).map_err(failure)?,
                None => Directory::default(),
            };
            directory.validate().map_err(failure)?;
            (World::Saved(session), directory)
        } else {
            let mut game = GameSession::new(project.clone());
            game.start(options()).map_err(failure)?;
            (World::Demo(game), Directory::default())
        };
        world
            .game()
            .server
            .set_region_scheduling(scheduling.clone())
            .map_err(failure)?;
        Ok(Self {
            scheduling,
            staging: None,
            started: Instant::now(),
            observing: false,
            metrics: Default::default(),
            operations: None,
            transport_metrics: None,
            player_limit: 64,
            world,
            project,
            directory: Arc::new(std::sync::RwLock::new(directory)),
            peers: HashMap::new(),
            tokens: HashMap::new(),
            content: HashMap::new(),
            content_bytes: 0,
            content_refs: Default::default(),
            spatial,
            content_revision,
            demo_spawns: 0,
        })
    }
    fn send(&mut self, peer: PeerId, msg: ServerMessage) {
        let started = self.observing.then(Instant::now);
        let encoded = codec::encode_server(&msg);
        if let Some(started) = started {
            self.metrics.encoding.record(started.elapsed());
        }
        let Ok(bytes) = encoded else {
            if let Some(stage) = &mut self.staging {
                stage.closed.insert(peer);
            } else {
                self.disconnect(peer);
            }
            return;
        };
        if let Some(stage) = &mut self.staging {
            if stage.closed.contains(&peer) {
                return;
            }
            let Some(state) = self.peers.get_mut(&peer) else {
                return;
            };
            let queued = state.output.max_capacity() - state.output.capacity();
            while state.queued_lengths.len() > queued {
                state.queued_lengths.pop_front();
            }
            let (count, size) = stage.totals.entry(peer).or_default();
            if queued + *count + 1 > MAX_SOCKET_QUEUE
                || state
                    .queued_lengths
                    .iter()
                    .sum::<usize>()
                    .saturating_add(*size)
                    .saturating_add(bytes.len())
                    > MAX_SERVER_BYTES
            {
                stage.closed.insert(peer);
                return;
            }
            *count += 1;
            *size += bytes.len();
            stage.packets.push((peer, bytes));
        } else {
            self.send_bytes(peer, bytes);
        }
    }
    fn send_bytes(&mut self, peer: PeerId, bytes: Vec<u8>) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let queued = state.output.max_capacity() - state.output.capacity();
        while state.queued_lengths.len() > queued {
            state.queued_lengths.pop_front();
        }
        let size = bytes.len();
        if state
            .queued_lengths
            .iter()
            .sum::<usize>()
            .saturating_add(size)
            > MAX_SERVER_BYTES
            || state.output.try_send(bytes).is_err()
        {
            self.metrics.slow_disconnects = self.metrics.slow_disconnects.saturating_add(1);
            self.disconnect(peer);
        } else {
            state.queued_lengths.push_back(size);
            self.metrics.packet_queue_peak = self.metrics.packet_queue_peak.max(queued + 1);
            self.metrics.packets_out = self.metrics.packets_out.saturating_add(1);
            self.metrics.bytes_out = self.metrics.bytes_out.saturating_add(size as u64);
        }
    }
    fn reject(&mut self, peer: PeerId, code: RejectCode) {
        if matches!(code, RejectCode::ServerBusy | RejectCode::Unavailable) {
            self.metrics.busy_rejections = self.metrics.busy_rejections.saturating_add(1);
        }
        if code == RejectCode::RateLimited {
            self.metrics.rate_rejections = self.metrics.rate_rejections.saturating_add(1);
        }
        self.send(peer, ServerMessage::Rejected { code });
    }
    fn summaries(&self, account: Uuid) -> Vec<CharacterSummary> {
        let World::Saved(session) = &self.world else {
            return vec![];
        };
        session
            .saved()
            .characters
            .iter()
            .filter(|c| c.account == Some(account))
            .map(|c| CharacterSummary {
                id: c.id,
                name: session
                    .saved()
                    .world_state
                    .regions
                    .iter()
                    .flat_map(|r| r.entities.iter().chain(&r.stashed))
                    .find(|e| e.id == c.entity)
                    .and_then(|e| e.attributes.get_str("name"))
                    .unwrap_or("Character")
                    .to_owned(),
            })
            .collect()
    }
    fn authorize(&mut self, peer: PeerId, account: Uuid) {
        // One live socket per account; a reconnect releases old control first.
        let previous: Vec<_> = self
            .peers
            .iter()
            .filter(|(id, p)| **id != peer && p.account == Some(account))
            .map(|(id, _)| *id)
            .collect();
        for id in previous {
            self.disconnect(id);
        }
        if let Some(p) = self.peers.get_mut(&peer) {
            p.account = Some(account);
        } else {
            return;
        }
        self.tokens
            .retain(|_, token| token.account != account && token.expires > Instant::now());
        let mut random = [0u8; 32];
        OsRng.fill_bytes(&mut random);
        let token = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let Some(security_revision) = self
            .directory
            .read()
            .unwrap()
            .account(account)
            .map(|a| a.security_revision)
        else {
            return;
        };
        self.tokens.insert(
            token.clone(),
            Token {
                account,
                security_revision,
                expires: Instant::now() + Duration::from_secs(3600),
            },
        );
        self.send(
            peer,
            ServerMessage::Authorized {
                account,
                token: Secret(token),
                characters: self.summaries(account),
            },
        );
    }
    fn disconnect(&mut self, peer: PeerId) {
        if let Some(p) = self.peers.remove(&peer)
            && let Some(connection) = p.connection
        {
            if matches!(self.world, World::Demo(_))
                && let Some(control) = self.world.game().server.player_control(connection)
            {
                // Retire disposable actors from collision/replication immediately.
                // All transient VM state is reclaimed when the demo becomes idle.
                for instance in &self.world.game().server.instances {
                    if let Some(ctx) = instance.lock().unwrap().context() {
                        ctx.lock().unwrap().stash_entity(control.entity_id);
                    }
                }
            }
            self.world.game().close_player_connection(connection);
        }
    }
    /// Bounded consecutive gameplay commands. Controls, ticks and content loads
    /// are barriers. No encoded reply/frame reaches a socket before commit.
    pub async fn command_group(&mut self, commands: Vec<Command>) -> Result<(), SaveError> {
        let started = self.observing.then(Instant::now);
        let result = self.command_group_inner(commands).await;
        if let Some(started) = started {
            self.metrics.action_group.record(started.elapsed());
        }
        result
    }
    async fn command_group_inner(&mut self, commands: Vec<Command>) -> Result<(), SaveError> {
        if commands.is_empty()
            || commands.len() > eldiron_runtime::persistence::MAX_GROUP_ACTIONS
            || commands.iter().any(|c| !c.is_action())
            || self.staging.is_some()
        {
            return Err(failure("Invalid action group"));
        }
        let saved = matches!(&self.world, World::Saved(_));
        if let World::Saved(session) = &mut self.world {
            session.begin_sequenced_group()?;
        }
        if saved {
            self.staging = Some(StagedOutput::default());
        }
        for command in commands {
            if let Err(error) = self.command(command).await {
                self.staging = None;
                if let World::Saved(session) = &mut self.world {
                    session.abort_sequenced_group();
                }
                return Err(error);
            }
        }
        if let World::Saved(session) = &mut self.world {
            if let Err(error) = session.finish_sequenced_group().await {
                self.staging = None;
                return Err(error);
            }
        }
        if let Some(stage) = self.staging.take() {
            for peer in &stage.closed {
                self.metrics.slow_disconnects = self.metrics.slow_disconnects.saturating_add(1);
                self.disconnect(*peer);
            }
            for (peer, bytes) in stage.packets {
                if !stage.closed.contains(&peer) {
                    self.send_bytes(peer, bytes);
                }
            }
        }
        Ok(())
    }
    pub async fn command(&mut self, cmd: Command) -> Result<(), SaveError> {
        let started = self.observing.then(Instant::now);
        // Joining/restoring can poll and publish several times without a timer
        // tick. Drain committed control-command frames before the next join so
        // a healthy existing subscriber cannot accumulate a burst of baselines.
        let publish = match &cmd {
            Command::Register { .. }
            | Command::Verified { .. }
            | Command::PasswordVerified { .. }
            | Command::Close(_) => true,
            Command::Message { message, .. } => !matches!(
                message,
                ClientMessage::Inputs { .. }
                    | ClientMessage::Ping { .. }
                    | ClientMessage::Action(_)
            ),
            _ => false,
        };
        let result = self.command_inner(cmd).await;
        let result = if result.is_ok() && publish {
            self.flush_frames()
        } else {
            result
        };
        if let Some(started) = started {
            self.metrics.command.record(started.elapsed());
        }
        result
    }
    async fn command_inner(&mut self, cmd: Command) -> Result<(), SaveError> {
        match cmd {
            Command::Open { peer, output } => {
                if self.peers.len() < 64 {
                    self.peers.insert(
                        peer,
                        Peer {
                            output,
                            hello: false,
                            account: None,
                            connection: None,
                            stream: None,
                            journal: Default::default(),
                            offers: HashMap::new(),
                            offer_context: None,
                            action_window: (Instant::now(), 0),
                            preloaded: Default::default(),
                            sent_content: Default::default(),
                            opened: Instant::now(),
                            queued_lengths: Default::default(),
                            last: Instant::now(),
                        },
                    );
                } else {
                    self.metrics.busy_rejections = self.metrics.busy_rejections.saturating_add(1);
                    if let Ok(bytes) = codec::encode_server(&ServerMessage::Rejected {
                        code: RejectCode::ServerBusy,
                    }) {
                        let _ = output.try_send(bytes);
                    }
                }
            }
            Command::Close(peer) => self.disconnect(peer),
            Command::Content { key, reply } => {
                let content = self.content.get(&key).cloned();
                if content.is_none() {
                    self.metrics.content_misses = self.metrics.content_misses.saturating_add(1);
                }
                let _ = reply.send(content);
            }
            Command::Verified {
                peer,
                account,
                security_revision,
            } => {
                if self
                    .peers
                    .get(&peer)
                    .is_some_and(|p| p.hello && p.account.is_none())
                    && self.world.mode() == HostMode::Persistent
                    && self
                        .directory
                        .read()
                        .unwrap()
                        .account(account)
                        .is_some_and(|a| a.security_revision == security_revision)
                {
                    self.authorize(peer, account);
                } else {
                    self.reject(peer, RejectCode::InvalidCredentials);
                }
            }
            Command::SecurityRejected { peer, code } => self.send(
                peer,
                ServerMessage::AccountUpdated(AccountUpdate::Rejected(code)),
            ),
            Command::PasswordContext { peer, reply } => {
                let result = self.password_context(peer);
                let _ = reply.send(result);
            }
            Command::PasswordVerified {
                peer,
                account,
                security_revision,
                hash,
            } => {
                self.password_verified(peer, account, security_revision, hash)
                    .await?
            }
            Command::Register {
                peer,
                username,
                hash,
            } => {
                if !self
                    .peers
                    .get(&peer)
                    .is_some_and(|p| p.hello && p.account.is_none())
                {
                    self.reject(peer, RejectCode::InvalidRequest);
                    return Ok(());
                }
                let mut directory = self.directory.read().unwrap().clone();
                if directory.accounts.contains_key(&username) || directory.accounts.len() >= 4096 {
                    self.reject(peer, RejectCode::Unavailable);
                    return Ok(());
                }
                let account = Account {
                    id: Uuid::new_v4(),
                    username: username.clone(),
                    password_hash: hash,
                    security_revision: 0,
                };
                directory.accounts.insert(username, account.clone());
                let World::Saved(session) = &mut self.world else {
                    self.reject(peer, RejectCode::WrongMode);
                    return Ok(());
                };
                if session.saved().requests.len()
                    >= eldiron_runtime::persistence::MAX_DURABLE_REQUESTS
                {
                    self.reject(peer, RejectCode::Unavailable);
                    return Ok(());
                }
                let data = serde_json::to_value(&directory).map_err(failure)?;
                session
                    .commit_operation(
                        Uuid::new_v4(),
                        &format!("account:{}", account.id),
                        move |saved| {
                            saved.host_data.insert("accounts.v1".into(), data);
                            Ok(serde_json::Value::Null)
                        },
                    )
                    .await?;
                *self.directory.write().unwrap() = directory;
                self.authorize(peer, account.id);
            }
            Command::Message { peer, message } => self.message(peer, message).await?,
        }
        Ok(())
    }
    async fn message(&mut self, peer: PeerId, message: ClientMessage) -> Result<(), SaveError> {
        self.metrics.messages_in = self.metrics.messages_in.saturating_add(1);
        let Some(p) = self.peers.get_mut(&peer) else {
            return Ok(());
        };
        p.last = Instant::now();
        if let ClientMessage::Hello { version } = message {
            if version != PROTOCOL_VERSION || p.hello {
                self.reject(peer, RejectCode::WrongVersion);
                self.disconnect(peer);
            } else {
                p.hello = true;
                self.send(
                    peer,
                    ServerMessage::Welcome {
                        version: PROTOCOL_VERSION,
                        mode: self.world.mode(),
                        content_revision: self.content_revision.clone(),
                        account_management: self.world.mode() == HostMode::Persistent,
                    },
                );
            }
            return Ok(());
        }
        if !p.hello {
            self.reject(peer, RejectCode::WrongVersion);
            return Ok(());
        }
        let account = p.account;
        let connection = p.connection;
        match message {
            ClientMessage::Ping { nonce } => self.send(peer, ServerMessage::Pong { nonce }),
            ClientMessage::Guest { name: chosen } => {
                if self.world.mode() != HostMode::Demo {
                    self.reject(peer, RejectCode::WrongMode);
                } else if connection.is_some() {
                    self.reject(peer, RejectCode::AlreadyPlaying);
                } else if self
                    .peers
                    .values()
                    .filter(|p| p.connection.is_some())
                    .count()
                    >= self.player_limit
                {
                    self.reject(peer, RejectCode::ServerBusy);
                } else if !name(&chosen) || self.demo_spawns >= 128 {
                    self.reject(peer, RejectCode::Unavailable);
                } else {
                    let conn = self
                        .world
                        .game()
                        .open_player_connection()
                        .map_err(failure)?;
                    self.peers.get_mut(&peer).unwrap().connection = Some(conn);
                    spawn(self.world.game(), conn, &chosen)?;
                    self.demo_spawns += 1;
                    self.activate(conn)?;
                }
            }
            ClientMessage::Resume { token } => {
                if self.world.mode() != HostMode::Persistent || account.is_some() {
                    self.reject(peer, RejectCode::WrongMode);
                } else if let Some(token) = self.tokens.remove(&token.0).filter(|t| {
                    t.expires > Instant::now()
                        && self
                            .directory
                            .read()
                            .unwrap()
                            .account(t.account)
                            .is_some_and(|a| a.security_revision == t.security_revision)
                }) {
                    self.authorize(peer, token.account);
                } else {
                    self.reject(peer, RejectCode::InvalidCredentials);
                }
            }
            ClientMessage::Login { .. } => self.reject(
                peer,
                if self.world.mode() == HostMode::Persistent {
                    RejectCode::InvalidCredentials
                } else {
                    RejectCode::WrongMode
                },
            ),
            ClientMessage::Register { .. } => self.reject(peer, RejectCode::WrongMode),
            ClientMessage::Characters => {
                if let Some(account) = account {
                    self.send(peer, ServerMessage::Characters(self.summaries(account)));
                } else {
                    self.reject(peer, RejectCode::AuthenticationRequired);
                }
            }
            ClientMessage::CreateCharacter {
                request,
                name: chosen,
            } => {
                let Some(account) = account else {
                    self.reject(peer, RejectCode::AuthenticationRequired);
                    return Ok(());
                };
                if !name(&chosen) || request.is_nil() {
                    self.reject(peer, RejectCode::InvalidRequest);
                    return Ok(());
                }
                let fingerprint = format!("create:{account}:{chosen}");
                let World::Saved(session) = &mut self.world else {
                    self.reject(peer, RejectCode::WrongMode);
                    return Ok(());
                };
                if let Some(receipt) = session.saved().requests.get(&request) {
                    if receipt.fingerprint != fingerprint {
                        self.reject(peer, RejectCode::InvalidRequest);
                        return Ok(());
                    }
                    let character =
                        serde_json::from_value(receipt.outcome.clone()).map_err(failure)?;
                    self.send(peer, ServerMessage::CharacterCreated { request, character });
                    return Ok(());
                }
                if connection.is_some() {
                    self.reject(peer, RejectCode::AlreadyPlaying);
                    return Ok(());
                }
                if self
                    .peers
                    .values()
                    .filter(|p| p.connection.is_some())
                    .count()
                    >= self.player_limit
                {
                    self.reject(peer, RejectCode::ServerBusy);
                    return Ok(());
                }
                if session
                    .saved()
                    .characters
                    .iter()
                    .filter(|c| c.account == Some(account))
                    .count()
                    >= 8
                    || session.saved().requests.len()
                        >= eldiron_runtime::persistence::MAX_DURABLE_REQUESTS
                {
                    self.reject(peer, RejectCode::Unavailable);
                    return Ok(());
                }
                let conn = session.game.open_player_connection().map_err(failure)?;
                self.peers.get_mut(&peer).unwrap().connection = Some(conn);
                let outcome = session
                    .commit_live_operation(
                        request,
                        &fingerprint,
                        move |game| Ok(serde_json::json!(spawn(game, conn, &chosen)?)),
                        move |saved, value| {
                            let entity =
                                value.as_u64().ok_or_else(|| failure("Missing new body"))? as u32;
                            let record = saved
                                .characters
                                .iter_mut()
                                .find(|c| c.entity == entity)
                                .ok_or_else(|| failure("New character not captured"))?;
                            record.account = Some(account);
                            Ok(serde_json::json!(record.id))
                        },
                    )
                    .await?;
                let character: Uuid = serde_json::from_value(outcome).map_err(failure)?;
                self.activate(conn)?;
                self.send(peer, ServerMessage::CharacterCreated { request, character });
            }
            ClientMessage::JoinCharacter { character } => {
                let Some(account) = account else {
                    self.reject(peer, RejectCode::AuthenticationRequired);
                    return Ok(());
                };
                if connection.is_some() {
                    self.reject(peer, RejectCode::AlreadyPlaying);
                    return Ok(());
                }
                if self
                    .peers
                    .values()
                    .filter(|p| p.connection.is_some())
                    .count()
                    >= self.player_limit
                {
                    self.reject(peer, RejectCode::ServerBusy);
                    return Ok(());
                }
                let World::Saved(session) = &mut self.world else {
                    self.reject(peer, RejectCode::WrongMode);
                    return Ok(());
                };
                if !session
                    .saved()
                    .characters
                    .iter()
                    .any(|c| c.id == character && c.account == Some(account))
                {
                    self.reject(peer, RejectCode::CharacterUnavailable);
                    return Ok(());
                }
                let conn = session.game.open_player_connection().map_err(failure)?;
                if session.join_character(conn, character).is_err() {
                    session.game.close_player_connection(conn);
                    self.reject(peer, RejectCode::CharacterUnavailable);
                } else {
                    self.peers.get_mut(&peer).unwrap().connection = Some(conn);
                    self.activate(conn)?;
                }
            }
            ClientMessage::Action(action) => self.game_action(peer, action).await?,
            ClientMessage::Inputs { frames } => {
                let Some(conn) = connection else {
                    self.reject(peer, RejectCode::AuthenticationRequired);
                    return Ok(());
                };
                for frame in frames {
                    if let Err(error) = self.world.game().submit_input_frame(conn, frame) {
                        use eldiron_runtime::FrameInputError;
                        // A baseline already sent on this ordered socket supersedes
                        // inputs generated before it arrived. Do not keep issuing
                        // new generations in response to that in-flight history.
                        if matches!(error, FrameInputError::NeedsBaseline)
                            && self
                                .world
                                .game()
                                .client_world(conn)
                                .is_some_and(|world| frame.generation < world.generation())
                        {
                            break;
                        }

                        if matches!(error, FrameInputError::Backpressure) {
                            break;
                        }
                        self.world.game().request_baseline(conn).map_err(failure)?;
                        self.reject(peer, RejectCode::InputResync);
                        break;
                    }
                }
            }
            ClientMessage::Resync => {
                if let Some(conn) = connection {
                    self.world.game().request_baseline(conn).map_err(failure)?;
                }
            }
            ClientMessage::LogoutAll => self.logout_all(peer).await?,
            ClientMessage::ChangePassword { .. } => self.send(
                peer,
                ServerMessage::AccountUpdated(AccountUpdate::Rejected(RejectCode::WrongMode)),
            ),
            ClientMessage::Logout => {
                if let Some(account) = account {
                    self.tokens.retain(|_, t| t.account != account);
                }
                self.disconnect(peer);
            }
            _ => self.reject(peer, RejectCode::InvalidRequest),
        }
        Ok(())
    }
    fn activate(&mut self, conn: PlayerConnectionId) -> Result<(), SaveError> {
        self.world
            .game()
            .enable_frame_input(conn)
            .map_err(failure)?;
        let subscriptions = self.spatial.clone();
        self.world
            .game()
            .set_spatial_subscriptions(conn, Some(subscriptions))
            .map_err(failure)?;
        self.world
            .game()
            .subscribe_replication(conn)
            .map_err(failure)?;
        self.open_action_stream(conn)
    }
    fn publish_content(
        &mut self,
        reference: eldiron_runtime::replication::RegionContentRef,
    ) -> Result<ContentHint, SaveError> {
        let game = self.world.game();
        let region = game
            .server
            .region_id_map
            .get(&reference.map)
            .ok_or_else(|| failure("Missing region"))?;
        let instance = game
            .server
            .instances
            .iter()
            .find(|i| i.lock().unwrap().id == *region)
            .ok_or_else(|| failure("Missing instance"))?;
        let context = instance
            .lock()
            .unwrap()
            .context()
            .ok_or_else(|| failure("Missing context"))?;
        let ctx = context.lock().unwrap();
        let physics_revision = ctx.collision_world.revision();
        if let Some(key) = self.content_refs.get(&(reference, physics_revision))
            && self.content.contains_key(key)
        {
            return Ok(ContentHint {
                reference,
                physics_revision,
                key: key.clone(),
            });
        }
        let bytes = codec::encode_content(&PhysicsContent {
            reference,
            physics_revision,
            name: ctx.map.name.clone(),
            mini: ctx.mapmini.clone(),
            collision: ctx.collision_world.clone(),
            geometry: RegionGeometry::capture(&ctx.map).map_err(failure)?,
        })
        .map_err(failure)?;
        drop(ctx);
        let key = format!("{:x}", Sha256::digest(&bytes));
        if !self.content.contains_key(&key) {
            // Content is immutable. Eviction returns HTTP 404; client requests a
            // fresh baseline rather than predicting with missing collision.
            if self.content_bytes + bytes.len() > 64 * 1024 * 1024 {
                self.content.clear();
                self.content_bytes = 0;
                self.content_refs.clear();
            }
            self.content_bytes += bytes.len();
            self.content.insert(key.clone(), Arc::new(bytes));
        }
        self.content_refs
            .insert((reference, physics_revision), key.clone());
        Ok(ContentHint {
            reference,
            physics_revision,
            key,
        })
    }
    pub async fn tick(&mut self, elapsed: Duration) -> Result<(), SaveError> {
        let started = self.observing.then(Instant::now);
        let accepted = elapsed.min(Duration::from_millis(250));
        self.metrics.wall_time_clamped_us = self.metrics.wall_time_clamped_us.saturating_add(
            elapsed
                .saturating_sub(accepted)
                .as_micros()
                .min(u64::MAX as u128) as u64,
        );
        let elapsed = accepted;
        let expired: Vec<_> = self
            .peers
            .iter()
            .filter(|(_, p)| {
                p.output.is_closed()
                    || p.last.elapsed() > Duration::from_secs(45)
                    || p.account.is_none()
                        && p.connection.is_none()
                        && p.opened.elapsed() > Duration::from_secs(15)
            })
            .map(|(id, _)| *id)
            .collect();
        for peer in expired {
            self.disconnect(peer);
        }
        self.tokens.retain(|_, t| t.expires > Instant::now());
        if matches!(self.world, World::Demo(_))
            && self.demo_spawns > 0
            && !self.peers.values().any(|p| p.connection.is_some())
        {
            let mut fresh = GameSession::new(self.project.clone());
            fresh.start(options()).map_err(failure)?;
            fresh.enable_diagnostics(self.observing);
            fresh
                .server
                .set_region_scheduling(self.scheduling.clone())
                .map_err(failure)?;
            self.world.game().stop();
            self.world = World::Demo(fresh);
            self.demo_spawns = 0;
            self.content.clear();
            self.content_refs.clear();
            self.content_bytes = 0;
        }
        match &mut self.world {
            World::Demo(game) => {
                game.advance(elapsed);
            }
            World::Saved(session) => {
                session.advance(elapsed).await?;
            }
        }
        self.flush_frames()?;
        self.housekeeping().await?;
        if let Some(started) = started {
            let elapsed = started.elapsed();
            self.metrics.ticks_over_budget = self
                .metrics
                .ticks_over_budget
                .saturating_add(u64::from(elapsed > Duration::from_millis(20)));
            self.metrics.tick.record(elapsed);
        }
        Ok(())
    }
    /// Publish only committed output, including offer replacement, before
    /// processing another gameplay request on the simulation owner.
    fn flush_frames(&mut self) -> Result<(), SaveError> {
        let peers: Vec<_> = self
            .peers
            .iter()
            .filter_map(|(id, p)| p.connection.map(|conn| (*id, conn)))
            .collect();
        for (peer, conn) in peers {
            let frames = match self.world.game().take_client_frames(conn) {
                Ok(frames) => frames,
                Err(
                    eldiron_runtime::ReplicationError::Backpressure
                    | eldiron_runtime::ReplicationError::NeedsBaseline
                    | eldiron_runtime::ReplicationError::UnknownConnection,
                ) => {
                    self.metrics.replica_disconnects =
                        self.metrics.replica_disconnects.saturating_add(1);
                    self.disconnect(peer);
                    continue;
                }
                Err(error) => return Err(failure(error)),
            };
            for frame in frames {
                use eldiron_runtime::replication::WorldUpdate;
                let refs: Vec<_> = match &frame.update {
                    WorldUpdate::Baseline(rs) => rs.iter().map(|r| r.content).collect(),
                    WorldUpdate::Delta(rs) => rs.iter().map(|r| r.content).collect(),
                };
                let mut content = vec![];
                for reference in refs {
                    let hint = self.publish_content(reference)?;
                    let baseline = matches!(frame.update, WorldUpdate::Baseline(_));
                    if let Some(state) = self.peers.get_mut(&peer)
                        && (baseline || state.sent_content.get(&reference) != Some(&hint.key))
                    {
                        state
                            .sent_content
                            .retain(|r, _| r.map != reference.map || *r == reference);
                        state.sent_content.insert(reference, hint.key.clone());
                        content.push(hint);
                    }
                }
                self.publish_offers(peer, &frame);
                self.send(peer, ServerMessage::Frame { frame, content });
            }
            // The outgoing packets own their transient events. Dedicated hosts
            // do not also retain an unread presentation/event mirror forever.
            let regions: Vec<_> = self
                .world
                .game()
                .client_world(conn)
                .map(|c| c.maps().map(|m| m.id).collect())
                .unwrap_or_default();
            for region in regions {
                let _ = self.world.game().take_connection_events(conn, region);
            }
            let prefetch = self.world.game().prefetched_content(conn);
            let mut preload = vec![];
            for reference in prefetch {
                if self
                    .world
                    .game()
                    .client_world(conn)
                    .and_then(|c| c.content_reference(reference.map))
                    == Some(reference)
                {
                    continue;
                }
                let hint = self.publish_content(reference)?;
                if let Some(state) = self.peers.get_mut(&peer)
                    && state.preloaded.get(&reference) != Some(&hint.key)
                {
                    state
                        .preloaded
                        .retain(|r, _| r.map != reference.map || *r == reference);
                    state.preloaded.insert(reference, hint.key.clone());
                    preload.push(hint);
                }
            }
            if !preload.is_empty() {
                self.send(peer, ServerMessage::Preload { content: preload });
            }
        }
        // The embedded local handle exists for compatibility, but a dedicated
        // process has no local presenter to consume its public event mirror.
        let local = self.world.game().server.local_connection();
        let regions: Vec<_> = self
            .world
            .game()
            .client_world(local)
            .map(|c| c.maps().map(|m| m.id).collect())
            .unwrap_or_default();
        for region in regions {
            let _ = self.world.game().take_connection_events(local, region);
        }
        Ok(())
    }
    pub async fn shutdown(&mut self) -> Result<(), SaveError> {
        let peers: Vec<_> = self.peers.keys().copied().collect();
        for id in peers {
            self.disconnect(id);
        }
        match &mut self.world {
            World::Saved(session) => session.shutdown().await,
            World::Demo(game) => {
                game.stop();
                Ok(())
            }
        }
    }
}
