//! Renderer-independent connection worker. Its only output is validated public
//! packets/content. It cannot create or advance an authoritative local world.
use eldiron_network::*;
use shared::project_play::MultiplayerSettings;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
pub enum Event {
    Packet(ServerMessage),
    Content(ContentHint, PhysicsContent),
    Region(eldiron_network::public_assets::PublicRegion),
    Failed(String),
}
pub struct Link {
    requests: SyncSender<ClientMessage>,
    events: Receiver<Event>,
    closed: Arc<AtomicBool>,
    diagnostics: Arc<Mutex<ContentStreamDiagnostics>>,
    failure: Arc<Mutex<Option<String>>>,
}
impl Link {
    pub fn open(
        settings: MultiplayerSettings,
        assets: Option<eldiron_network::public_assets::RegionAssets>,
    ) -> Self {
        let (requests, input) = mpsc::sync_channel(64);
        let (output, events) = mpsc::sync_channel(64);
        let closed = Arc::new(AtomicBool::new(false));
        let flag = closed.clone();
        let diagnostics = Arc::new(Mutex::new(ContentStreamDiagnostics::default()));
        let observed = diagnostics.clone();
        let failure = Arc::new(Mutex::new(None));
        let terminal = failure.clone();
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
                .and_then(|runtime| {
                    runtime.block_on(run_native(
                        settings, assets, input, &output, &flag, &observed,
                    ))
                });
            if let Err(error) = result {
                *terminal.lock().unwrap() = Some(error.clone());
                let _ = output.try_send(Event::Failed(error));
            }
        });
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) =
                run_browser(settings, assets, input, &output, &flag, &observed).await
            {
                *terminal.lock().unwrap() = Some(error.clone());
                let _ = output.try_send(Event::Failed(error));
            }
        });
        Self {
            requests,
            events,
            closed,
            diagnostics,
            failure,
        }
    }
    pub fn send(&self, message: ClientMessage) -> Result<(), String> {
        self.requests
            .try_send(message)
            .map_err(|_| "Connection unavailable or busy".into())
    }
    pub fn diagnostics(&self) -> ContentStreamDiagnostics {
        self.diagnostics.lock().unwrap().clone()
    }
    pub fn poll(&self) -> Option<Event> {
        match self.events.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Event::Failed(
                self.failure
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| "Disconnected; reconnect to continue".into()),
            )),
        }
    }
}
impl Drop for Link {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}
fn deliver(
    stream: &mut ContentStream,
    gate: &mut eldiron_network::public_asset_gate::PublicAssetGate,
    endpoint: &str,
    output: &SyncSender<Event>,
    observed: &Mutex<ContentStreamDiagnostics>,
    pending: &mut Option<Event>,
) -> Result<(), String> {
    // Backpressure the bounded UI channel without blocking socket progress.
    // Keep one unsent event so a full channel cannot discard a frame/receipt.
    for _ in 0..8 {
        let event = if let Some(event) = pending.take() {
            event
        } else {
            use eldiron_network::public_asset_gate::AssetDelivery;
            match gate.poll(stream, endpoint)? {
                Some(AssetDelivery::Content(ContentDelivery::Content(hint, package))) => {
                    Event::Content(hint, package)
                }
                Some(AssetDelivery::Content(ContentDelivery::Packet(packet))) => {
                    Event::Packet(packet)
                }
                Some(AssetDelivery::Region(region)) => Event::Region(region),
                None => break,
            }
        };
        match output.try_send(event) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(event)) => {
                *pending = Some(event);
                break;
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err("Connection consumer closed".into());
            }
        }
    }
    *observed.lock().unwrap() = stream.diagnostics();
    Ok(())
}
/// A final control reply may be followed immediately by WebSocket close.
/// Drain verified FIFO output before exposing EOF, with a bounded stall lifetime.
#[derive(Default)]
struct Closing {
    error: Option<(String, instant::Instant)>,
}
impl Closing {
    fn observe(&mut self, error: String) {
        self.error
            .get_or_insert_with(|| (error, instant::Instant::now()));
    }
    fn check(
        &self,
        ended: bool,
        stream: &ContentStream,
        pending: &Option<Event>,
    ) -> Result<(), String> {
        if let Some((error, started)) = &self.error {
            if (ended && stream.diagnostics().queued_packets == 0 && pending.is_none())
                || started.elapsed() >= std::time::Duration::from_secs(10)
            {
                return Err(error.clone());
            }
        }
        Ok(())
    }
}
#[cfg(not(target_arch = "wasm32"))]
async fn run_native(
    settings: MultiplayerSettings,
    assets: Option<eldiron_network::public_assets::RegionAssets>,
    input: Receiver<ClientMessage>,
    output: &SyncSender<Event>,
    closed: &AtomicBool,
    observed: &Mutex<ContentStreamDiagnostics>,
) -> Result<(), String> {
    use eldiron_network::native;
    let cancel = async {
        while !closed.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    let mut socket = tokio::select! {
        socket=native::connect(&settings.endpoint,settings.allow_insecure_loopback)=>socket?,
        _=cancel=>return Ok(()),
    };
    native::send(
        &mut socket,
        &ClientMessage::Hello {
            version: PROTOCOL_VERSION,
        },
    )
    .await?;
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    let mut heartbeat = 0;
    let mut stream = ContentStream::default();
    let mut gate = eldiron_network::public_asset_gate::PublicAssetGate::new(assets);
    let mut pending = None;
    let mut closing = Closing::default();
    let mut ended = false;
    loop {
        tokio::select! {
            _=interval.tick()=>{
                deliver(&mut stream,&mut gate,&settings.endpoint,output,observed,&mut pending)?;
                if closed.load(Ordering::Relaxed) {
                    while let Ok(message)=input.try_recv() {if matches!(message,ClientMessage::Logout){let _=native::send(&mut socket,&message).await;}}
                    return Ok(());
                }
                closing.check(ended,&stream,&pending)?;
                if closing.error.is_none() {
                    for _ in 0..64 {let Ok(message)=input.try_recv() else {break;};if let Err(error)=native::send(&mut socket,&message).await {closing.observe(error);break;}}
                    heartbeat+=1;if heartbeat==500&&closing.error.is_none() {heartbeat=0;if let Err(error)=native::send(&mut socket,&ClientMessage::Ping{nonce:0}).await {closing.observe(error);}}
                }
            },
            packet=native::receive(&mut socket), if !ended=>{
                match packet {Ok(packet)=>stream.push(&settings.endpoint,packet)?,Err(error)=>{ended=true;closing.observe(error);}}
                *observed.lock().unwrap()=stream.diagnostics();
            }
        }
    }
}
#[cfg(target_arch = "wasm32")]
async fn run_browser(
    settings: MultiplayerSettings,
    assets: Option<eldiron_network::public_assets::RegionAssets>,
    input: Receiver<ClientMessage>,
    output: &SyncSender<Event>,
    closed: &AtomicBool,
    observed: &Mutex<ContentStreamDiagnostics>,
) -> Result<(), String> {
    use eldiron_network::browser;
    let socket =
        browser::BrowserSocket::connect(&settings.endpoint, settings.allow_insecure_loopback)
            .map_err(js_error)?;
    let mut hello = false;
    let mut heartbeat = 0;
    let mut stream = ContentStream::default();
    let mut gate = eldiron_network::public_asset_gate::PublicAssetGate::new(assets);
    let mut pending = None;
    let mut closing = Closing::default();
    loop {
        deliver(
            &mut stream,
            &mut gate,
            &settings.endpoint,
            output,
            observed,
            &mut pending,
        )?;
        if closed.load(Ordering::Relaxed) {
            while let Ok(message) = input.try_recv() {
                if matches!(message, ClientMessage::Logout) {
                    let _ = socket.send(&message);
                }
            }
            return Ok(());
        }
        // Close can already be visible while the browser inbox holds its final
        // reply. Always drain that inbox, including after ready() becomes false.
        let mut inbox_drained = false;
        for _ in 0..64 {
            let Some(packet) = socket.receive() else {
                inbox_drained = true;
                break;
            };
            stream.push(&settings.endpoint, packet)?;
        }
        if socket.closed() {
            closing.observe("Disconnected from server".into());
        }
        closing.check(socket.closed() && inbox_drained, &stream, &pending)?;
        if socket.ready() && closing.error.is_none() {
            if !hello {
                socket
                    .send(&ClientMessage::Hello {
                        version: PROTOCOL_VERSION,
                    })
                    .map_err(js_error)?;
                hello = true;
            }
            for _ in 0..64 {
                let Ok(message) = input.try_recv() else {
                    break;
                };
                if let Err(error) = socket.send(&message) {
                    closing.observe(js_error(error));
                    break;
                }
            }
            heartbeat += 1;
            if heartbeat == 500 {
                heartbeat = 0;
                if let Err(error) = socket.send(&ClientMessage::Ping { nonce: 0 }) {
                    closing.observe(js_error(error));
                }
            }
        }
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10);
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}
#[cfg(target_arch = "wasm32")]
fn js_error(error: wasm_bindgen::JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "Browser connection failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_account_reply_survives_eof_and_ui_backpressure() {
        let mut stream = ContentStream::default();
        stream
            .push(
                "",
                ServerMessage::AccountUpdated(AccountUpdate::PasswordChanged),
            )
            .unwrap();
        let mut closing = Closing::default();
        closing.observe("closed".into());
        let (mut gate, mut pending) = (
            eldiron_network::public_asset_gate::PublicAssetGate::default(),
            None,
        );
        assert!(closing.check(true, &stream, &pending).is_ok());
        let (output, receiver) = mpsc::sync_channel(1);
        output
            .send(Event::Packet(ServerMessage::Pong { nonce: 1 }))
            .unwrap();
        let observed = Mutex::new(ContentStreamDiagnostics::default());
        deliver(&mut stream, &mut gate, "", &output, &observed, &mut pending).unwrap();
        assert!(pending.is_some());
        assert!(closing.check(true, &stream, &pending).is_ok());
        receiver.try_recv().unwrap();
        deliver(&mut stream, &mut gate, "", &output, &observed, &mut pending).unwrap();
        assert!(closing.check(true, &stream, &pending).is_err());
        assert!(matches!(
            receiver.try_recv().unwrap(),
            Event::Packet(ServerMessage::AccountUpdated(
                AccountUpdate::PasswordChanged
            ))
        ));
    }
    #[test]
    fn full_ui_channel_retries_every_unsent_event_in_order() {
        let (output, receiver) = mpsc::sync_channel(1);
        let observed = Mutex::new(ContentStreamDiagnostics::default());
        let mut stream = ContentStream::default();
        let mut gate = eldiron_network::public_asset_gate::PublicAssetGate::new(None);
        let mut pending = None;
        for sequence in 1..=200 {
            stream
                .push(
                    "",
                    ServerMessage::CharacterCreated {
                        request: theframework::prelude::Uuid::nil(),
                        character: theframework::prelude::Uuid::from_u128(sequence),
                    },
                )
                .unwrap();
        }
        for expected in 1..=200 {
            deliver(&mut stream, &mut gate, "", &output, &observed, &mut pending).unwrap();
            // Repeated polling while the consumer remains full is harmless.
            deliver(&mut stream, &mut gate, "", &output, &observed, &mut pending).unwrap();
            assert!(
                matches!(receiver.try_recv().unwrap(),Event::Packet(ServerMessage::CharacterCreated{character,..}) if character.as_u128()==expected)
            );
        }
        deliver(&mut stream, &mut gate, "", &output, &observed, &mut pending).unwrap();
        assert!(pending.is_none());
        assert!(receiver.try_recv().is_err());
        assert_eq!(stream.diagnostics().queued_packets, 0);
    }
}
