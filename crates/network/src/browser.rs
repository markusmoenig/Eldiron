//! Browser-owned WebSocket: no native sockets, threads, or TLS shim on WASM.
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{MessageEvent, WebSocket};
pub struct BrowserSocket {
    socket: WebSocket,
    inbox: Rc<RefCell<VecDeque<ServerMessage>>>,
    failed: Rc<Cell<bool>>,
    _message: Closure<dyn FnMut(MessageEvent)>,
    _error: Closure<dyn FnMut(web_sys::Event)>,
}
impl BrowserSocket {
    pub fn connect(endpoint: &str, allow_loopback: bool) -> Result<Self, JsValue> {
        let insecure = endpoint.strip_prefix("ws://").is_some_and(|rest| {
            let authority = rest.split('/').next().unwrap_or("");
            authority == "[::1]"
                || authority
                    .strip_prefix("[::1]:")
                    .is_some_and(|port| port.parse::<u16>().is_ok())
                || matches!(authority.split(':').next(), Some("localhost" | "127.0.0.1"))
        });
        if endpoint.contains(['?', '#', '@'])
            || !(endpoint.starts_with("wss://") || allow_loopback && insecure)
        {
            return Err(JsValue::from_str("Secure WebSocket endpoint required"));
        }
        let socket = WebSocket::new_with_str(endpoint, SUBPROTOCOL)?;
        socket.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let inbox = Rc::new(RefCell::new(VecDeque::new()));
        let failed = Rc::new(Cell::new(false));
        let queue = inbox.clone();
        let error = failed.clone();
        let ws = socket.clone();
        let message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let result = event
                .data()
                .dyn_into::<js_sys::ArrayBuffer>()
                .ok()
                .and_then(|buffer| {
                    if buffer.byte_length() as usize > MAX_SERVER_BYTES {
                        return None;
                    }
                    codec::decode_server(&js_sys::Uint8Array::new(&buffer).to_vec()).ok()
                });
            if let Some(msg) = result
                && queue.borrow().len() < MAX_SOCKET_QUEUE
            {
                queue.borrow_mut().push_back(msg);
            } else {
                error.set(true);
                let _ = ws.close();
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
        let error = failed.clone();
        let onerror =
            Closure::wrap(Box::new(move |_: web_sys::Event| error.set(true))
                as Box<dyn FnMut(web_sys::Event)>);
        socket.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        Ok(Self {
            socket,
            inbox,
            failed,
            _message: message,
            _error: onerror,
        })
    }
    pub fn ready(&self) -> bool {
        self.socket.ready_state() == WebSocket::OPEN
            && !self.failed.get()
            && self.socket.protocol() == SUBPROTOCOL
    }
    pub fn closed(&self) -> bool {
        self.failed.get() || self.socket.ready_state() >= WebSocket::CLOSING
    }
    pub fn send(&self, msg: &ClientMessage) -> Result<(), JsValue> {
        if !self.ready() || self.socket.buffered_amount() as usize > MAX_CLIENT_BYTES * 4 {
            return Err(JsValue::from_str("Socket unavailable or backpressured"));
        }
        let bytes = codec::encode_client(msg).map_err(|_| JsValue::from_str("Invalid request"))?;
        self.socket.send_with_u8_array(&bytes)
    }
    pub fn receive(&self) -> Option<ServerMessage> {
        self.inbox.borrow_mut().pop_front()
    }
}
pub async fn fetch_content(endpoint: &str, hint: &ContentHint) -> Result<PhysicsContent, JsValue> {
    let abort = crate::content_task::FetchAbort::new()?;
    fetch_content_with_signal(endpoint, hint, &abort.signal()).await
}
pub(crate) async fn fetch_content_with_signal(
    endpoint: &str,
    hint: &ContentHint,
    signal: &web_sys::AbortSignal,
) -> Result<PhysicsContent, JsValue> {
    let url = crate::remote::content_url(endpoint, &hint.key).map_err(|e| JsValue::from_str(&e))?;
    let options = web_sys::RequestInit::new();
    options.set_signal(Some(signal));
    let response = wasm_bindgen_futures::JsFuture::from(
        web_sys::window()
            .ok_or_else(|| JsValue::from_str("Browser window required"))?
            .fetch_with_str_and_init(&url, &options),
    )
    .await?
    .dyn_into::<web_sys::Response>()?;
    if !response.ok() {
        return Err(JsValue::from_str(
            "Content unavailable; reconnect to resync",
        ));
    }
    if response
        .headers()
        .get("content-length")?
        .and_then(|n| n.parse::<usize>().ok())
        .is_some_and(|n| n > MAX_CONTENT_BYTES)
    {
        return Err(JsValue::from_str("Content too large"));
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer()?)
        .await?
        .dyn_into::<js_sys::ArrayBuffer>()?;
    if buffer.byte_length() as usize > MAX_CONTENT_BYTES {
        return Err(JsValue::from_str("Content too large"));
    }
    crate::remote::verify_content(hint, &js_sys::Uint8Array::new(&buffer).to_vec())
        .map_err(|e| JsValue::from_str(&e))
}
impl Drop for BrowserSocket {
    fn drop(&mut self) {
        self.socket.set_onmessage(None);
        self.socket.set_onerror(None);
        let _ = self.socket.close();
    }
}
/// Small accountless WASM connection viewer. The graphical Player uses the
/// same BrowserSocket and ActionClient with its own account and gameplay UI.
#[wasm_bindgen]
pub struct DemoClient {
    socket: BrowserSocket,
    endpoint: String,
    name: String,
    client: Option<crate::remote::RemoteClient>,
    hello: bool,
    joined: bool,
    status: String,
    nonce: u32,
    actions: ActionClient,
    last_action: Option<ActionResult>,
    input_cadence: InputCadence,
    content: ContentStream,
    staged: Vec<(ContentHint, PhysicsContent)>,
}
#[wasm_bindgen]
impl DemoClient {
    #[wasm_bindgen(constructor)]
    pub fn new(
        endpoint: String,
        name: String,
        allow_loopback: bool,
    ) -> Result<DemoClient, JsValue> {
        Ok(Self {
            socket: BrowserSocket::connect(&endpoint, allow_loopback)?,
            endpoint,
            name,
            client: None,
            hello: false,
            joined: false,
            status: "Connecting".into(),
            nonce: 0,
            actions: ActionClient::default(),
            last_action: None,
            input_cadence: Default::default(),
            content: ContentStream::default(),
            staged: vec![],
        })
    }
    pub fn intent(&mut self, key: &str) {
        use eldiron_runtime::MovementIntent;
        if let Some(client) = &mut self.client {
            client.motion.set_intent(match key {
                "w" => MovementIntent::Forward,
                "s" => MovementIntent::Backward,
                "a" => MovementIntent::StrafeLeft,
                "d" => MovementIntent::StrafeRight,
                _ => MovementIntent::Stop,
            });
        }
    }
    /// Submit the same bounded gameplay DTO used by native clients. Actor,
    /// account, price, callback and distance cannot be supplied in this JSON.
    pub fn request(&mut self, request_json: &str) -> Result<(), JsValue> {
        if request_json.len() > 8192 {
            return Err(JsValue::from_str("Request too large"));
        }
        let request = serde_json::from_str::<GameRequest>(request_json)
            .map_err(|_| JsValue::from_str("Invalid gameplay request"))?;
        self.actions
            .enqueue(request)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        self.last_action = None;
        Ok(())
    }
    pub async fn pump(&mut self, elapsed_ms: f64) -> Result<(), JsValue> {
        if self.socket.closed() {
            self.status = "Disconnected".into();
            return Err(JsValue::from_str("Disconnected"));
        }
        if !self.socket.ready() {
            return Ok(());
        }
        if !self.hello {
            self.socket.send(&ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            })?;
            self.hello = true;
        }
        for _ in 0..MAX_SOCKET_QUEUE {
            let Some(msg) = self.socket.receive() else {
                break;
            };
            self.content
                .push(&self.endpoint, msg)
                .map_err(|e| JsValue::from_str(&e))?;
        }
        for _ in 0..8 {
            let Some(delivery) = self
                .content
                .poll(&self.endpoint)
                .map_err(|e| JsValue::from_str(&e))?
            else {
                break;
            };
            let msg = match delivery {
                ContentDelivery::Content(hint, package) => {
                    self.staged.push((hint, package));
                    continue;
                }
                ContentDelivery::Packet(packet) => packet,
            };
            match msg {
                ServerMessage::ActionStream {
                    stream,
                    next_sequence,
                } => self.actions.stream_ready(stream, next_sequence),
                ServerMessage::Offers {
                    world,
                    generation,
                    region,
                    offers,
                } => self.actions.offers(world, generation, region, offers),
                ServerMessage::ActionReply(reply) => {
                    if let Some(result) = self.actions.reply(&reply) {
                        self.last_action = Some(result);
                    }
                }
                ServerMessage::Welcome {
                    mode: HostMode::Demo,
                    ..
                } if !self.joined => {
                    self.socket.send(&ClientMessage::Guest {
                        name: self.name.clone(),
                    })?;
                    self.joined = true;
                    self.status = "Joining demo".into();
                }
                ServerMessage::Frame { frame, .. } => {
                    let mut client = self
                        .client
                        .take()
                        .unwrap_or_else(|| crate::remote::RemoteClient::new(&frame));
                    for (hint, package) in self.staged.drain(..) {
                        client
                            .install(&hint, package)
                            .map_err(|e| JsValue::from_str(&e))?;
                    }
                    client.receive(&frame).map_err(|e| JsValue::from_str(&e))?;
                    self.actions.observe_frame(&frame);
                    self.client = Some(client);
                    self.status = "Playing demo".into();
                }
                ServerMessage::Rejected {
                    code: RejectCode::InputResync,
                } => {
                    if let Some(client) = &mut self.client {
                        client.motion.resume();
                    }
                }
                ServerMessage::Rejected { code } => {
                    return Err(JsValue::from_str(&format!("Server: {code:?}")));
                }
                ServerMessage::Welcome { .. } => {
                    return Err(JsValue::from_str("This demo client requires a --demo host"));
                }
                _ => {}
            }
        }
        let elapsed = if elapsed_ms.is_finite() {
            elapsed_ms.clamp(0., 100.)
        } else {
            0.
        };
        if let Some(client) = &mut self.client {
            let msg = client.advance(std::time::Duration::from_secs_f64(elapsed / 1000.));
            match msg {
                ClientMessage::Inputs { frames } => {
                    if let Some(message) = self
                        .input_cadence
                        .advance(std::time::Duration::from_secs_f64(elapsed / 1000.), frames)
                    {
                        self.socket.send(&message)?;
                    }
                }
                message => self.socket.send(&message)?,
            }
        }
        if let Some(action) = self
            .actions
            .advance(std::time::Duration::from_secs_f64(elapsed / 1000.))
        {
            self.socket.send(&action)?;
        }
        self.nonce = self.nonce.wrapping_add(1);
        if self.nonce % 100 == 0 {
            self.socket
                .send(&ClientMessage::Ping { nonce: self.nonce })?;
        }
        Ok(())
    }
    pub fn state(&self) -> String {
        let mut actors = vec![];
        let project = eldiron_runtime::Project::default();
        if let Some(client) = &self.client
            && let Some(view) = client.motion.view(&project, &client.assets, None)
            && let Some(map) = view.map
        {
            for entity in &map.entities {
                actors.push(serde_json::json!({"id":entity.id,"name":entity.attributes.get_str("name").unwrap_or("Actor"),"x":entity.position.x,"z":entity.position.z,"mode":entity.get_mode(),"owned":Some(entity.id)==client.motion.authority().controlled_entity()}));
            }
        }
        serde_json::json!({"status":self.status,"actors":actors,"action":self.last_action,"cached_regions":self.content.cached_regions(),"cached_bytes":self.content.cached_bytes(),"content_delivery":self.content.diagnostics(),"replica":self.client.as_ref().map(|c| serde_json::json!({"tick":c.motion.authority().tick(),"sequence":c.motion.authority().sequence(),"generation":c.motion.authority().generation(),"motion":c.motion.diagnostics(),"snapshot":c.motion.authority().motion(),"authority_position":c.motion.authority().current_map().and_then(|m| m.entities.iter().find(|e| Some(e.id)==c.motion.authority().controlled_entity())).map(|e| [e.position.x,e.position.y,e.position.z])}))})
            .to_string()
    }
    pub fn logout(&self) {
        let _ = self.socket.send(&ClientMessage::Logout);
    }
}
