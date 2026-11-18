use eldiron_network::{native, *};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use theframework_core::Uuid;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_rustls::rustls;
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub world: Uuid,
    pub generation: u64,
    pub region: Uuid,
    pub body: u32,
}
pub struct Peer {
    tx: mpsc::Sender<ClientMessage>,
    rx: mpsc::Receiver<ServerMessage>,
    pending: std::collections::VecDeque<ServerMessage>,
    pub context: Arc<Mutex<Option<Context>>>,
    pub frames: Arc<AtomicU64>,
    task: JoinHandle<()>,
}
impl Peer {
    pub async fn connect(endpoint: &str, tls: Arc<rustls::ClientConfig>) -> Self {
        let mut socket = native::connect_with_connector(
            endpoint,
            false,
            Some(tokio_tungstenite::Connector::Rustls(tls)),
        )
        .await
        .unwrap();
        native::send(
            &mut socket,
            &ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )
        .await
        .unwrap();
        let (tx, mut input) = mpsc::channel(64);
        let (out, rx) = mpsc::channel(128);
        let context = Arc::new(Mutex::new(None));
        let latest = context.clone();
        let frames = Arc::new(AtomicU64::new(0));
        let count = frames.clone();
        let task = tokio::spawn(async move {
            let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
            loop {
                tokio::select! {
                    message=input.recv()=>{let Some(message)=message else{return;};if native::send(&mut socket,&message).await.is_err(){return;}},
                    _=heartbeat.tick()=>{if native::send(&mut socket,&ClientMessage::Ping{nonce:0}).await.is_err(){return;}},
                    packet=native::receive(&mut socket)=>{
                        let Ok(packet)=packet else{return;};
                        if let ServerMessage::Frame{frame,..}=&packet{
                            // No party/container grants are established by this fixture.
                            let check=|e:&eldiron_runtime::replication::EntitySnapshot|{
                                if Some(e.id)!=frame.controlled_entity {assert!(e.inventory.is_none()&&e.wallet.is_none(),"foreign private body state crossed WSS");}
                            };
                            match &frame.update{
                                eldiron_runtime::replication::WorldUpdate::Baseline(regions)=>{for r in regions{for e in &r.entities{check(e);}}},
                                eldiron_runtime::replication::WorldUpdate::Delta(regions)=>{for r in regions{for e in &r.entities{check(e);}}},
                            }
                            if let (Some(region),Some(body))=(frame.current_region,frame.controlled_entity){*latest.lock().unwrap()=Some(Context{world:frame.world,generation:frame.generation,region,body});}
                            count.fetch_add(1,Ordering::Relaxed);
                        } else if !matches!(packet,ServerMessage::Pong{nonce:0}|ServerMessage::Offers{..}|ServerMessage::Preload{..}) {
                            let encoded=codec::encode_server(&packet).unwrap();assert!(!encoded.windows(b"$argon2id$".len()).any(|w|w==b"$argon2id$"));
                            if out.try_send(packet).is_err(){return;}
                        }
                    }
                }
            }
        });
        let mut peer = Self {
            tx,
            rx,
            pending: Default::default(),
            context,
            frames,
            task,
        };
        peer.until(|m| matches!(m, ServerMessage::Welcome { .. }))
            .await;
        peer
    }
    pub async fn try_send(&self, message: ClientMessage) -> Result<(), ()> {
        tokio::time::timeout(Duration::from_secs(3), self.tx.send(message))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())
    }
    pub async fn closed(&mut self) {
        tokio::time::timeout(Duration::from_secs(10), async {
            while self.rx.recv().await.is_some() {}
        })
        .await
        .unwrap();
        (&mut self.task).await.expect("peer worker panicked");
    }
    pub async fn send(&self, message: ClientMessage) {
        tokio::time::timeout(Duration::from_secs(5), self.tx.send(message))
            .await
            .unwrap()
            .unwrap();
    }
    pub async fn until(&mut self, test: impl Fn(&ServerMessage) -> bool) -> ServerMessage {
        if let Some(index) = self.pending.iter().position(&test) {
            return self.pending.remove(index).unwrap();
        }
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let m = self.rx.recv().await.expect("connection closed");
                if test(&m) {
                    return m;
                }
                assert!(self.pending.len() < 128);
                self.pending.push_back(m);
            }
        })
        .await
        .expect("network response timed out")
    }
    pub async fn credentials(&mut self, user: &str, register: bool) -> String {
        let password = Secret("release test password".into());
        self.send(if register {
            ClientMessage::Register {
                username: user.into(),
                password,
            }
        } else {
            ClientMessage::Login {
                username: user.into(),
                password,
            }
        })
        .await;
        let ServerMessage::Authorized { token, .. } = self
            .until(|m| {
                matches!(
                    m,
                    ServerMessage::Authorized { .. } | ServerMessage::Rejected { .. }
                )
            })
            .await
        else {
            panic!("authentication failed")
        };
        token.0
    }

    pub async fn join(&mut self, character: Uuid) -> u64 {
        *self.context.lock().unwrap() = None;
        self.send(ClientMessage::JoinCharacter { character }).await;
        self.stream(character).await
    }
    pub async fn stream(&mut self, character: Uuid) -> u64 {
        let ServerMessage::ActionStream {
            stream,
            next_sequence,
        } = self
            .until(|m| {
                matches!(
                    m,
                    ServerMessage::ActionStream { .. } | ServerMessage::Rejected { .. }
                )
            })
            .await
        else {
            panic!("join failed")
        };
        assert_eq!(stream, character);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while self.context.lock().unwrap().is_none() {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        next_sequence
    }

    pub async fn reply(&mut self, seq: u64) -> ActionReply {
        let ServerMessage::ActionReply(reply) = self
            .until(|m| matches!(m,ServerMessage::ActionReply(r) if r.sequence==seq))
            .await
        else {
            unreachable!()
        };
        reply
    }
    pub async fn ping(&mut self, nonce: u32) {
        self.send(ClientMessage::Ping { nonce }).await;
        self.until(|m| matches!(m,ServerMessage::Pong{nonce:n} if *n==nonce))
            .await;
    }
}
// Abort-on-drop owns the socket and releases it even when an assertion fails.
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
