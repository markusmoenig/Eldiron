use crate::{
    auth,
    host::{Command, Host},
};
use eldiron_network::*;
use futures_util::{SinkExt, StreamExt};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};
use theframework_core::Uuid;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::TcpListener,
    sync::{Semaphore, mpsc, oneshot},
    time::timeout,
};
use tokio_rustls::{TlsAcceptor, rustls};
use tokio_tungstenite::tungstenite::{
    Message,
    handshake::server::{ErrorResponse, Request, Response},
    protocol::WebSocketConfig,
};

pub struct TransportConfig {
    pub tls: Option<Arc<rustls::ServerConfig>>,
    pub origins: Vec<String>,
}
#[derive(Default)]
struct Limits {
    ips: HashMap<IpAddr, (Instant, u32, u32)>,
}
impl Limits {
    fn take(&mut self, ip: IpAddr, auth: bool) -> bool {
        self.ips
            .retain(|_, v| v.0.elapsed() < Duration::from_secs(60));
        if !self.ips.contains_key(&ip) && self.ips.len() >= 4096 {
            return false;
        }
        let value = self.ips.entry(ip).or_insert((Instant::now(), 0, 0));
        let count = if auth { &mut value.2 } else { &mut value.1 };
        let max = if auth { 10 } else { 60 };
        if *count >= max {
            return false;
        }
        *count += 1;
        true
    }
}
/// One world owner. Socket workers never touch simulation or saves. A storage
/// failure ends the host before another frame can publish uncommitted state.
pub async fn serve<F: std::future::Future<Output = ()>>(
    listener: TcpListener,
    mut host: Host,
    config: TransportConfig,
    shutdown: F,
) -> Result<(), String> {
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    if config.tls.is_none() && !address.ip().is_loopback() {
        return Err("Plain WS is restricted to loopback".into());
    }
    let (tx, mut rx) = mpsc::channel(128);
    // The serving future owns every socket task. Drop/abort/fail-closed exit
    // must not leave old-world sockets or permits alive in an embedding host.
    let mut connections = tokio::task::JoinSet::new();
    let limits = Arc::new(Mutex::new(Limits::default()));
    // Keep HTTP/TLS headroom even when all 64 gameplay slots are occupied.
    let slots = Arc::new(Semaphore::new(128));
    let game_slots = Arc::new(Semaphore::new(64));
    let metrics = Arc::new(crate::metrics::TransportMetrics::default());
    host.transport_metrics = Some(metrics.clone());
    let hashes = Arc::new(Semaphore::new(4));
    let directory = host.directory.clone();
    let identity = host.identity.clone();
    let mode = host.world.mode();
    let dummy =
        tokio::task::spawn_blocking(|| auth::hash("dummy missing account password").unwrap())
            .await
            .map_err(|e| e.to_string())?;
    let config = Arc::new(config);
    let mut clock = tokio::time::interval(Duration::from_millis(20));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last = Instant::now();
    tokio::pin!(shutdown);
    loop {
        while connections.try_join_next().is_some() {}
        tokio::select! {
            _=&mut shutdown=>{
                let result=host.shutdown().await.map_err(|e|e.to_string());
                connections.abort_all();
                while connections.join_next().await.is_some() {}
                host.write_metrics_now();return result;
            },
            _=clock.tick()=>{
                let now=Instant::now(); let elapsed=now.duration_since(last); last=now;
                if let Err(error)=host.tick(elapsed).await {host.write_metrics_now();return Err(error.to_string());}
            },
            Some(command)=rx.recv()=>{if let Err(error)=process_commands(&mut host, command, &mut rx).await {host.write_metrics_now();return Err(error.to_string());}},
            Some(_)=connections.join_next(), if !connections.is_empty()=>{},
            accepted=listener.accept()=>{
                let (stream,address)=accepted.map_err(|e| e.to_string())?;
                if !limits.lock().unwrap().take(address.ip(),false) {metrics.refused.fetch_add(1,std::sync::atomic::Ordering::Relaxed);continue;}
                if connections.len()>=128 {metrics.refused.fetch_add(1,std::sync::atomic::Ordering::Relaxed);continue;}
                let Ok(slot)=slots.clone().try_acquire_owned() else {metrics.refused.fetch_add(1,std::sync::atomic::Ordering::Relaxed);continue;};
                metrics.accepted.fetch_add(1,std::sync::atomic::Ordering::Relaxed);metrics.active.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
                let tx=tx.clone(); let config=config.clone(); let directory=directory.clone();
                let limits=limits.clone(); let hashes=hashes.clone(); let dummy=dummy.clone();let metrics=metrics.clone();let game_slots=game_slots.clone();
                let identity=identity.clone();
                let active=ActiveConnection(metrics.clone());
                connections.spawn(async move {
                    let _slot=slot;let _active=active;
                    let _=stream.set_nodelay(true);
                    if let Some(tls)=&config.tls {
                        if let Ok(Ok(stream))=timeout(Duration::from_secs(10),TlsAcceptor::from(tls.clone()).accept(stream)).await {
                            let _=connection(stream,address,tx,config,directory,mode,limits,hashes,dummy,game_slots,metrics,identity).await;
                        }
                    } else { let _=connection(stream,address,tx,config,directory,mode,limits,hashes,dummy,game_slots,metrics,identity).await; }
                });
            }
        }
    }
}
// Wait at most two milliseconds after the first action. A non-action command
// ends collection and executes after this group, preserving channel order.
async fn collect_actions(
    first: Command,
    rx: &mut mpsc::Receiver<Command>,
) -> (Vec<Command>, Option<Command>) {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(2);
    let mut actions = vec![first];
    while actions.len() < eldiron_runtime::persistence::MAX_GROUP_ACTIONS {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Some(next)) if next.is_action() => actions.push(next),
            Ok(Some(barrier)) => return (actions, Some(barrier)),
            _ => break,
        }
    }
    (actions, None)
}
async fn process_commands(
    host: &mut Host,
    command: Command,
    rx: &mut mpsc::Receiver<Command>,
) -> Result<(), eldiron_runtime::persistence::SaveError> {
    if command.is_action() && host.world.mode() == HostMode::Persistent {
        let (actions, barrier) = collect_actions(command, rx).await;
        host.command_group(actions).await?;
        if let Some(barrier) = barrier {
            host.command(barrier).await?;
        }
        Ok(())
    } else {
        host.command(command).await
    }
}
struct ActiveConnection(Arc<crate::metrics::TransportMetrics>);
impl Drop for ActiveConnection {
    fn drop(&mut self) {
        self.0
            .active
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
async fn dispatch(
    tx: &mpsc::Sender<Command>,
    metrics: &crate::metrics::TransportMetrics,
    command: Command,
) -> Result<(), mpsc::error::SendError<Command>> {
    let depth = (tx.max_capacity() - tx.capacity() + 1).min(tx.max_capacity()) as u64;
    metrics
        .command_queue_peak
        .fetch_max(depth, std::sync::atomic::Ordering::Relaxed);
    tx.send(command).await
}
// Replay the bounded HTTP prefix into tungstenite after routing /content.
struct Prefixed<S> {
    prefix: std::io::Cursor<Vec<u8>>,
    stream: S,
}
impl<S: AsyncRead + Unpin> AsyncRead for Prefixed<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let pos = self.prefix.position() as usize;
        if pos < self.prefix.get_ref().len() {
            let n = buf.remaining().min(self.prefix.get_ref().len() - pos);
            buf.put_slice(&self.prefix.get_ref()[pos..pos + n]);
            self.prefix.set_position((pos + n) as u64);
            Poll::Ready(Ok(()))
        } else {
            Pin::new(&mut self.stream).poll_read(cx, buf)
        }
    }
}
impl<S: AsyncWrite + Unpin> AsyncWrite for Prefixed<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, data)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
async fn connection<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    mut stream: S,
    address: SocketAddr,
    tx: mpsc::Sender<Command>,
    config: Arc<TransportConfig>,
    directory: auth::SharedDirectory,
    mode: HostMode,
    limits: Arc<Mutex<Limits>>,
    hashes: Arc<Semaphore>,
    dummy: String,
    game_slots: Arc<Semaphore>,
    metrics: Arc<crate::metrics::TransportMetrics>,
    identity: Option<Arc<crate::oidc::Broker>>,
) -> Result<(), ()> {
    let mut prefix = Vec::new();
    timeout(Duration::from_secs(10), async {
        loop {
            let b = stream.read_u8().await.map_err(|_| ())?;
            prefix.push(b);
            if prefix.ends_with(b"\r\n\r\n") {
                return Ok::<_, ()>(());
            }
            if prefix.len() >= 8192 {
                return Err::<(), ()>(());
            }
        }
    })
    .await
    .map_err(|_| ())??;
    let header = std::str::from_utf8(&prefix).map_err(|_| ())?;
    let path = header
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|line| line.strip_suffix(" HTTP/1.1"))
        .ok_or(())?;
    if let Some(key) = path.strip_prefix("/content/") {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err::<(), ()>(());
        }
        let origin = header
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case("origin"))
            .map(|(_, v)| v.trim());
        if origin.is_some_and(|o| !config.origins.iter().any(|allowed| allowed == o)) {
            return Err::<(), ()>(());
        }
        let cors = origin
            .map(|o| format!("Access-Control-Allow-Origin: {o}\r\nVary: Origin\r\n"))
            .unwrap_or_default();
        let (reply, rx) = oneshot::channel();
        dispatch(
            &tx,
            &metrics,
            Command::Content {
                key: key.into(),
                reply,
            },
        )
        .await
        .map_err(|_| ())?;
        let bytes = timeout(Duration::from_secs(5), rx)
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?;
        let (status, bytes) = bytes
            .map(|b| ("200 OK", b))
            .unwrap_or_else(|| ("404 Not Found", Arc::new(vec![])));
        let headers = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/vnd.eldiron.physics\r\nContent-Length: {}\r\nCache-Control: public, max-age=31536000, immutable\r\nX-Content-Type-Options: nosniff\r\n{cors}Connection: close\r\n\r\n",
            bytes.len()
        );
        timeout(Duration::from_secs(10), async {
            stream.write_all(headers.as_bytes()).await?;
            stream.write_all(&bytes).await?;
            stream.shutdown().await
        })
        .await
        .map_err(|_| ())?
        .map_err(|_| ())?;
        metrics
            .http_tx_bytes
            .fetch_add(bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
        return Ok(());
    }
    if path != "/play" {
        return Err::<(), ()>(());
    }
    let Ok(_game_slot) = game_slots.try_acquire_owned() else {
        metrics
            .refused
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _=timeout(Duration::from_secs(1),stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")).await;
        return Ok(());
    };
    let origins = config.origins.clone();
    let callback =
        move |req: &Request, mut response: Response| -> Result<Response, ErrorResponse> {
            let allowed_origin = req
                .headers()
                .get("origin")
                .map(|s| {
                    s.to_str()
                        .ok()
                        .is_some_and(|o| origins.iter().any(|x| x == o))
                })
                .unwrap_or(true);
            let protocol = req
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.split(',').any(|s| s.trim() == SUBPROTOCOL));
            if !allowed_origin || !protocol {
                let mut error = ErrorResponse::new(Some("Eldiron origin/protocol required".into()));
                *error.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
                return Err(error);
            }
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", SUBPROTOCOL.parse().unwrap());
            Ok(response)
        };
    let ws = WebSocketConfig::default()
        .max_message_size(Some(MAX_CLIENT_BYTES))
        .max_frame_size(Some(MAX_CLIENT_BYTES));
    let socket = timeout(
        Duration::from_secs(10),
        tokio_tungstenite::accept_hdr_async_with_config(
            Prefixed {
                prefix: std::io::Cursor::new(prefix),
                stream,
            },
            callback,
            Some(ws),
        ),
    )
    .await
    .map_err(|_| ())?
    .map_err(|_| ())?;
    let peer = Uuid::new_v4();
    let (out, mut outgoing) = mpsc::channel(MAX_SOCKET_QUEUE);
    dispatch(&tx, &metrics, Command::Open { peer, output: out })
        .await
        .map_err(|_| ())?;
    let (mut sink, mut source) = socket.split();
    let mut identity_jobs = tokio::task::JoinSet::new();
    let reader = async {
        let mut hello = false;
        let mut messages = 0;
        let mut window = Instant::now();
        loop {
            let incoming = tokio::select! {
                result = timeout(Duration::from_secs(30), source.next()) => result,
                Some(_) = identity_jobs.join_next(), if !identity_jobs.is_empty() => continue,
            }
            .map_err(|_| ())?
            .ok_or(())?
            .map_err(|_| ())?;
            if window.elapsed() > Duration::from_secs(1) {
                window = Instant::now();
                messages = 0;
            }
            messages += 1;
            if messages > 120 {
                return Err::<(), ()>(());
            }
            let message = match incoming {
                Message::Binary(bytes) => {
                    metrics
                        .websocket_rx_bytes
                        .fetch_add(bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
                    codec::decode_client(&bytes).map_err(|_| {
                        metrics
                            .malformed
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        ()
                    })?
                }
                Message::Pong(_) => continue,
                _ => return Err::<(), ()>(()),
            };
            if !hello {
                if !matches!(
                    message,
                    ClientMessage::Hello {
                        version: PROTOCOL_VERSION
                    }
                ) {
                    return Err::<(), ()>(());
                }
                hello = true;
            }
            match message {
                ClientMessage::UnlinkExternal {
                    provider,
                    current_password,
                } => {
                    if !limits.lock().unwrap().take(address.ip(), true) {
                        return Err(());
                    }
                    let Some(issuer) = identity
                        .as_ref()
                        .and_then(|broker| broker.issuer(&provider))
                    else {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::SecurityRejected {
                                peer,
                                code: RejectCode::Unavailable,
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                        continue;
                    };
                    let (reply, context) = oneshot::channel();
                    dispatch(&tx, &metrics, Command::PasswordContext { peer, reply })
                        .await
                        .map_err(|_| ())?;
                    let account = match context.await.map_err(|_| ())? {
                        Ok(account) => account,
                        Err(code) => {
                            dispatch(&tx, &metrics, Command::SecurityRejected { peer, code })
                                .await
                                .map_err(|_| ())?;
                            continue;
                        }
                    };
                    let id = account.id;
                    let security_revision = account.security_revision;
                    let permit = hashes.clone().try_acquire_owned().map_err(|_| ())?;
                    let verified = tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        account.local_password
                            && auth::verify(&current_password.0, &account.password_hash)
                    })
                    .await
                    .map_err(|_| ())?;
                    let command = if verified {
                        Command::ExternalUnlinkVerified {
                            peer,
                            account: id,
                            security_revision,
                            issuer,
                        }
                    } else {
                        Command::SecurityRejected {
                            peer,
                            code: RejectCode::InvalidCredentials,
                        }
                    };
                    dispatch(&tx, &metrics, command).await.map_err(|_| ())?;
                }
                ClientMessage::ExternalLogin {
                    provider,
                    current_password,
                } => {
                    let Some(broker) = identity.clone().filter(|_| mode == HostMode::Persistent)
                    else {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::Message {
                                peer,
                                message: ClientMessage::ExternalLogin {
                                    provider,
                                    current_password,
                                },
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                        continue;
                    };
                    if !limits.lock().unwrap().take(address.ip(), true) {
                        return Err(());
                    }
                    let permit = if identity_jobs.is_empty() {
                        broker.slots.clone().try_acquire_owned().ok()
                    } else {
                        None
                    };
                    let Some(permit) = permit else {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::SecurityRejected {
                                peer,
                                code: RejectCode::ServerBusy,
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                        continue;
                    };
                    let (reply, context) = oneshot::channel();
                    dispatch(&tx, &metrics, Command::ExternalContext { peer, reply })
                        .await
                        .map_err(|_| ())?;
                    let context = match context.await.map_err(|_| ())? {
                        Ok(context) => context,
                        Err(code) => {
                            dispatch(&tx, &metrics, Command::SecurityRejected { peer, code })
                                .await
                                .map_err(|_| ())?;
                            continue;
                        }
                    };
                    let tx = tx.clone();
                    let metrics = metrics.clone();
                    let hashes = hashes.clone();
                    identity_jobs.spawn(async move {
                        let _permit = permit;
                        let attempt = context.attempt;
                        let result = async {
                            if let Some(account) = &context.link {
                                let password = current_password.ok_or(())?;
                                let hash = account.password_hash.clone();
                                let permit = hashes.clone().try_acquire_owned().map_err(|_| ())?;
                                let verified = tokio::task::spawn_blocking(move || {
                                    let _permit = permit;
                                    auth::verify(&password.0, &hash)
                                })
                                .await
                                .map_err(|_| ())?;
                                if !account.local_password || !verified {
                                    return Err(());
                                }
                            } else if current_password.is_some() {
                                return Err(());
                            }
                            let (pending, challenge) = broker.begin(&provider).await?;
                            dispatch(
                                &tx,
                                &metrics,
                                Command::ExternalChallenge {
                                    peer,
                                    attempt,
                                    challenge,
                                },
                            )
                            .await
                            .map_err(|_| ())?;
                            let identity = broker.finish(pending).await?;
                            let placeholder_hash = if context.link.is_none() {
                                let permit = hashes.clone().try_acquire_owned().map_err(|_| ())?;
                                tokio::task::spawn_blocking(move || {
                                    let _permit = permit;
                                    auth::external_placeholder_hash()
                                })
                                .await
                                .map_err(|_| ())??
                            } else {
                                String::new()
                            };
                            dispatch(
                                &tx,
                                &metrics,
                                Command::ExternalVerified {
                                    peer,
                                    context,
                                    identity,
                                    placeholder_hash,
                                },
                            )
                            .await
                            .map_err(|_| ())
                        }
                        .await;
                        if result.is_err() {
                            let _ =
                                dispatch(&tx, &metrics, Command::ExternalFailed { peer, attempt })
                                    .await;
                        }
                    });
                }
                ClientMessage::CancelExternalLogin => {
                    identity_jobs.abort_all();
                    while identity_jobs.join_next().await.is_some() {}
                    dispatch(
                        &tx,
                        &metrics,
                        Command::Message {
                            peer,
                            message: ClientMessage::CancelExternalLogin,
                        },
                    )
                    .await
                    .map_err(|_| ())?;
                }
                ClientMessage::Login { username, password } => {
                    if mode != HostMode::Persistent {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::Message {
                                peer,
                                message: ClientMessage::Login { username, password },
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                        continue;
                    }
                    if !limits.lock().unwrap().take(address.ip(), true) {
                        return Err::<(), ()>(());
                    }
                    let account = auth::username(&username)
                        .and_then(|name| directory.read().unwrap().accounts.get(&name).cloned());
                    let encoded = account
                        .as_ref()
                        .map(|a| a.password_hash.clone())
                        .unwrap_or_else(|| dummy.clone());
                    let permit = hashes.clone().try_acquire_owned().map_err(|_| {
                        metrics
                            .hash_busy
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        ()
                    })?;
                    let verified = tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        auth::verify(&password.0, &encoded)
                    })
                    .await
                    .map_err(|_| ())?;
                    if verified && let Some(account) = account.filter(|a| a.local_password) {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::Verified {
                                peer,
                                account: account.id,
                                security_revision: account.security_revision,
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                    } else {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::Message {
                                peer,
                                message: ClientMessage::Login {
                                    username: String::new(),
                                    password: Secret(String::new()),
                                },
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                    }
                }
                ClientMessage::ChangePassword {
                    current_password,
                    new_password,
                } => {
                    if mode != HostMode::Persistent {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::SecurityRejected {
                                peer,
                                code: RejectCode::WrongMode,
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                        continue;
                    }
                    if !limits.lock().unwrap().take(address.ip(), true) {
                        return Err::<(), ()>(());
                    }
                    let (reply, context) = tokio::sync::oneshot::channel();
                    dispatch(&tx, &metrics, Command::PasswordContext { peer, reply })
                        .await
                        .map_err(|_| ())?;
                    let account = match context.await.map_err(|_| ())? {
                        Ok(a) => a,
                        Err(code) => {
                            dispatch(&tx, &metrics, Command::SecurityRejected { peer, code })
                                .await
                                .map_err(|_| ())?;
                            continue;
                        }
                    };
                    let permit = hashes.clone().try_acquire_owned().map_err(|_| {
                        metrics
                            .hash_busy
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        ()
                    })?;
                    let revision = account.security_revision;
                    let id = account.id;
                    let result = tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        if !account.local_password
                            || !auth::verify(&current_password.0, &account.password_hash)
                        {
                            return Err(RejectCode::InvalidCredentials);
                        }
                        auth::hash(&new_password.0).map_err(|_| RejectCode::InvalidRequest)
                    })
                    .await
                    .map_err(|_| ())?;
                    let command = match result {
                        Ok(hash) => Command::PasswordVerified {
                            peer,
                            account: id,
                            security_revision: revision,
                            hash,
                        },
                        Err(code) => Command::SecurityRejected { peer, code },
                    };
                    dispatch(&tx, &metrics, command).await.map_err(|_| ())?;
                }
                ClientMessage::LogoutAll => {
                    if mode == HostMode::Persistent
                        && !limits.lock().unwrap().take(address.ip(), true)
                    {
                        return Err::<(), ()>(());
                    }
                    dispatch(
                        &tx,
                        &metrics,
                        Command::Message {
                            peer,
                            message: ClientMessage::LogoutAll,
                        },
                    )
                    .await
                    .map_err(|_| ())?;
                }
                ClientMessage::Register { username, password } => {
                    if mode != HostMode::Persistent {
                        dispatch(
                            &tx,
                            &metrics,
                            Command::Message {
                                peer,
                                message: ClientMessage::Register { username, password },
                            },
                        )
                        .await
                        .map_err(|_| ())?;
                        continue;
                    }
                    if !limits.lock().unwrap().take(address.ip(), true) {
                        return Err::<(), ()>(());
                    }
                    let username = auth::username(&username).ok_or(())?;
                    let permit = hashes.clone().try_acquire_owned().map_err(|_| {
                        metrics
                            .hash_busy
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        ()
                    })?;
                    let hash = tokio::task::spawn_blocking(move || {
                        let _permit = permit;
                        auth::hash(&password.0)
                    })
                    .await
                    .map_err(|_| ())??;
                    dispatch(
                        &tx,
                        &metrics,
                        Command::Register {
                            peer,
                            username,
                            hash,
                        },
                    )
                    .await
                    .map_err(|_| ())?;
                }
                message => dispatch(&tx, &metrics, Command::Message { peer, message })
                    .await
                    .map_err(|_| ())?,
            }
        }
    };
    let writer = async {
        while let Some(bytes) = outgoing.recv().await {
            let length = bytes.len();
            let started = Instant::now();
            let result = timeout(
                Duration::from_secs(5),
                sink.send(Message::Binary(bytes.into())),
            )
            .await;
            metrics.write.lock().unwrap().record(started.elapsed());
            result.map_err(|_| ())?.map_err(|_| ())?;
            metrics
                .websocket_tx_bytes
                .fetch_add(length as u64, std::sync::atomic::Ordering::Relaxed);
        }
        let _ = sink.close().await;
        Ok::<_, ()>(())
    };
    tokio::select! {_=reader=>{},_=writer=>{}}
    // Await cleanup rather than losing logout on a full command queue.
    let _ = dispatch(&tx, &metrics, Command::Close(peer)).await;
    Ok(())
}
pub fn load_tls(
    cert: &std::path::Path,
    key: &std::path::Path,
) -> Result<Arc<rustls::ServerConfig>, String> {
    let cert = std::fs::read(cert).map_err(|e| e.to_string())?;
    let key = std::fs::read(key).map_err(|e| e.to_string())?;
    let certs = rustls_pemfile::certs(&mut &cert[..])
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let key = rustls_pemfile::private_key(&mut &key[..])
        .map_err(|e| e.to_string())?
        .ok_or("No private key")?;
    rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn collector_bounds_groups_and_preserves_control_barriers() {
        let (tx, mut rx) = mpsc::channel(16);
        let action = || Command::Message {
            peer: Uuid::new_v4(),
            message: ClientMessage::Action(GameAction {
                stream: Uuid::new_v4(),
                sequence: 1,
                world: Uuid::new_v4(),
                generation: 1,
                region: Uuid::new_v4(),
                request: GameRequest::Intent("look".into()),
            }),
        };
        tx.send(action()).await.unwrap();
        tx.send(Command::Close(Uuid::nil())).await.unwrap();
        tx.send(action()).await.unwrap();
        let (group, barrier) = collect_actions(action(), &mut rx).await;
        assert_eq!(group.len(), 2);
        assert!(matches!(barrier, Some(Command::Close(_))));
        assert!(rx.try_recv().unwrap().is_action());
        for _ in 0..8 {
            tx.send(action()).await.unwrap();
        }
        let (group, barrier) = collect_actions(action(), &mut rx).await;
        assert_eq!(group.len(), 8);
        assert!(barrier.is_none());
        assert!(rx.try_recv().unwrap().is_action());
        let (group, barrier) = collect_actions(action(), &mut rx).await;
        assert_eq!(group.len(), 1);
        assert!(barrier.is_none());
    }
    #[tokio::test]
    async fn full_gameplay_slots_refuse_new_play_but_keep_http_content_available() {
        let metrics = Arc::new(crate::metrics::TransportMetrics::default());
        for (path, expected) in [
            ("/play".to_owned(), "503 Service Unavailable"),
            (format!("/content/{}", "a".repeat(64)), "200 OK"),
        ] {
            let (server, mut client) = tokio::io::duplex(8192);
            let (tx, mut commands) = mpsc::channel(1);
            let task = tokio::spawn(connection(
                server,
                "127.0.0.1:1234".parse().unwrap(),
                tx,
                Arc::new(TransportConfig {
                    tls: None,
                    origins: vec![],
                }),
                Default::default(),
                HostMode::Demo,
                Default::default(),
                Arc::new(Semaphore::new(4)),
                String::new(),
                Arc::new(Semaphore::new(0)),
                metrics.clone(),
                None,
            ));
            client
                .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
                .await
                .unwrap();
            if path.starts_with("/content/") {
                let Command::Content { reply, .. } = commands.recv().await.unwrap() else {
                    panic!()
                };
                reply
                    .send(Some(Arc::new(b"content payload".to_vec())))
                    .unwrap();
            }
            let mut response = String::new();
            client.read_to_string(&mut response).await.unwrap();
            assert!(response.contains(expected), "{response}");
            task.await.unwrap().unwrap();
        }
        assert_eq!(metrics.snapshot().refused, 1);
        assert_eq!(metrics.snapshot().http_payload_bytes_sent, 15);
    }
}
