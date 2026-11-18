//! Test-only opaque TCP impairment. TLS stays end-to-end with hostname/cert
//! verification. TCP remains ordered/reliable; cuts model lost connections,
//! never arbitrary WebSocket message reordering or claimed UDP packet loss.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
    task::JoinHandle,
};
#[derive(Default)]
pub struct Counters {
    pub bytes: AtomicU64,
    pub fragments: AtomicU64,
    pub connections: AtomicU64,
    pub active: AtomicU64,
    pub peak_active: AtomicU64,
}
#[derive(Clone, Copy)]
pub struct Profile {
    pub latency_ms: u64,
    pub jitter_ms: u64,
    pub bytes_per_second: u64,
}
impl Profile {
    pub fn wan() -> Self {
        Self {
            latency_ms: 45,
            jitter_ms: 45,
            bytes_per_second: 192 * 1024,
        }
    }
    pub fn distant() -> Self {
        Self {
            latency_ms: 120,
            jitter_ms: 80,
            bytes_per_second: 96 * 1024,
        }
    }
}
struct Active(Arc<Counters>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}
pub struct Relay {
    pub port: u16,
    pub hold: Arc<AtomicBool>,
    pub counters: Arc<Counters>,
    cut: watch::Sender<u64>,
    task: JoinHandle<()>,
    upstream: Arc<AtomicU16>,
    hold_up: Arc<AtomicBool>,
    profile: watch::Sender<Profile>,
}
impl Relay {
    pub async fn open(upstream: u16) -> Self {
        Self::with_profile(
            upstream,
            Profile {
                latency_ms: 5,
                jitter_ms: 20,
                bytes_per_second: 0,
            },
        )
        .await
    }
    pub async fn with_profile(upstream: u16, profile: Profile) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let hold = Arc::new(AtomicBool::new(false));
        let counters = Arc::new(Counters::default());
        let hold_up = Arc::new(AtomicBool::new(false));
        let held_up = hold_up.clone();
        let upstream = Arc::new(AtomicU16::new(upstream));
        let target = upstream.clone();
        let (profile, profiles) = watch::channel(profile);
        let (cut, _) = watch::channel(0u64);
        let cuts = cut.clone();
        let h = hold.clone();
        let c = counters.clone();
        let task = tokio::spawn(async move {
            let mut peers = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    Some(_)=peers.join_next(), if !peers.is_empty()=>{},
                    accepted=listener.accept()=>{
                        let Ok((client,_)) = accepted else { return; };
                        if peers.len() >= 128 {drop(client);continue;}
                        c.connections.fetch_add(1,Ordering::Relaxed);
                        let active = c.active.fetch_add(1,Ordering::SeqCst)+1;
                        c.peak_active.fetch_max(active,Ordering::SeqCst);
                        let guard = Active(c.clone());
                        let target = target.load(Ordering::SeqCst);
                        let up = held_up.clone();let profile = profiles.clone();
                        let mut cut = cuts.subscribe(); let hold = h.clone(); let stats = c.clone();
                        peers.spawn(async move {
                            let _guard = guard;
                            let Ok(server) = tokio::net::TcpStream::connect(("127.0.0.1",target)).await else { return; };
                            let _=client.set_nodelay(true);let _=server.set_nodelay(true);
                            let (cr,cw)=client.into_split();let (sr,sw)=server.into_split();
                            tokio::select! {
                                _=cut.changed()=>{},
                                _=forward(cr,sw,Some(up),stats.clone(),profile.clone())=>{},
                                _=forward(sr,cw,Some(hold),stats,profile)=>{},
                            }
                        });
                    }
                }
            }
        });
        Self {
            port,
            hold,
            counters,
            cut,
            task,
            upstream,
            hold_up,
            profile,
        }
    }
    pub fn endpoint(&self) -> String {
        format!("wss://localhost:{}/play", self.port)
    }
    pub fn pause(&self) {
        self.hold.store(true, Ordering::SeqCst);
    }
    pub fn set_profile(&self, profile: Profile) {
        self.profile.send_replace(profile);
    }
    pub fn pause_both(&self) {
        self.hold_up.store(true, Ordering::SeqCst);
        self.pause();
    }
    pub fn resume(&self) {
        self.hold_up.store(false, Ordering::SeqCst);
        self.hold.store(false, Ordering::SeqCst);
    }
    pub fn retarget(&self, port: u16) {
        self.upstream.store(port, Ordering::SeqCst);
        self.cut();
    }
    pub fn cut(&self) {
        self.cut.send_modify(|n| *n += 1);
        self.resume();
    }
}
impl Drop for Relay {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn forward<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    mut input: R,
    mut output: W,
    hold: Option<Arc<AtomicBool>>,
    stats: Arc<Counters>,
    profile: watch::Receiver<Profile>,
) {
    let (sender, mut queue) =
        tokio::sync::mpsc::channel::<(Vec<u8>, tokio::time::Instant, Profile)>(8);
    let reader = async move {
        let mut buf = [0u8; 8192];
        let mut turn = 0u64;
        loop {
            let Ok(n) = input.read(&mut buf).await else {
                return;
            };
            if n == 0 {
                return;
            }
            let policy = *profile.borrow();
            let ready = tokio::time::Instant::now()
                + Duration::from_millis(policy.latency_ms + policy.jitter_ms * (turn % 5) / 4);
            if sender
                .send((buf[..n].to_vec(), ready, policy))
                .await
                .is_err()
            {
                return;
            }
            turn += 1;
        }
    };
    let writer = async move {
        let mut next = tokio::time::Instant::now();
        let mut turn = 0usize;
        while let Some((bytes, ready, profile)) = queue.recv().await {
            // Propagation delay pipelines; serialization bandwidth has its own
            // clock. Bounded queued reads preserve TCP backpressure and order.
            tokio::time::sleep_until(ready.max(next)).await;
            let sent = tokio::time::Instant::now();
            for fragment in bytes.chunks([31, 127, 509, 1024][turn % 4]) {
                while hold.as_ref().is_some_and(|v| v.load(Ordering::SeqCst)) {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                if output.write_all(fragment).await.is_err() {
                    return;
                }
                stats
                    .bytes
                    .fetch_add(fragment.len() as u64, Ordering::Relaxed);
                stats.fragments.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
            }
            next = if profile.bytes_per_second == 0 {
                sent
            } else {
                sent + Duration::from_secs_f64(bytes.len() as f64 / profile.bytes_per_second as f64)
            };
            turn += 1;
        }
    };
    tokio::pin!(reader, writer);
    tokio::select! {_=&mut writer=>{},_=&mut reader=>{writer.await;}}
}
