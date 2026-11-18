use crate::*;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    time::{Duration, timeout},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub async fn connect(endpoint: &str, allow_loopback: bool) -> Result<Socket, String> {
    connect_with_connector(endpoint, allow_loopback, None).await
}
/// Optional private-CA trust for managed hosts/tests. Certificate and hostname
/// verification remain enabled by the supplied rustls client configuration.
pub async fn connect_with_connector(
    endpoint: &str,
    allow_loopback: bool,
    connector: Option<tokio_tungstenite::Connector>,
) -> Result<Socket, String> {
    let mut request = endpoint
        .into_client_request()
        .map_err(|_| "Invalid endpoint")?;
    let uri = request.uri();
    let local = matches!(uri.host(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if uri.query().is_some()
        || uri.authority().is_none_or(|a| a.as_str().contains('@'))
        || !(uri.scheme_str() == Some("wss")
            || allow_loopback && local && uri.scheme_str() == Some("ws"))
    {
        return Err("Use wss://host/play; ws is allowed only for explicit loopback testing".into());
    }
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", SUBPROTOCOL.parse().unwrap());
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_SERVER_BYTES))
        .max_frame_size(Some(MAX_SERVER_BYTES));
    let (socket, response) = timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async_tls_with_config(request, Some(config), true, connector),
    )
    .await
    .map_err(|_| "Connection timed out")?
    .map_err(|e| format!("WebSocket: {e}"))?;
    if response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|s| s.to_str().ok())
        != Some(SUBPROTOCOL)
    {
        return Err("Server did not select Eldiron protocol".into());
    }
    Ok(socket)
}
pub async fn send(socket: &mut Socket, msg: &ClientMessage) -> Result<(), String> {
    let bytes = codec::encode_client(msg).map_err(|e| e.to_string())?;
    timeout(
        Duration::from_secs(5),
        socket.send(Message::Binary(bytes.into())),
    )
    .await
    .map_err(|_| "Send timed out")?
    .map_err(|e| e.to_string())
}
pub async fn receive(socket: &mut Socket) -> Result<ServerMessage, String> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Binary(bytes))) => {
                return codec::decode_server(&bytes).map_err(|e| e.to_string());
            }
            Some(Ok(Message::Ping(_))) => socket.flush().await.map_err(|e| e.to_string())?,
            Some(Ok(Message::Pong(_))) => {}
            _ => return Err("Connection closed or invalid message".into()),
        }
    }
}
pub async fn fetch_content(endpoint: &str, hint: &ContentHint) -> Result<PhysicsContent, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    fetch_content_with_client(endpoint, hint, &client).await
}
pub async fn fetch_content_with_client(
    endpoint: &str,
    hint: &ContentHint,
    client: &reqwest::Client,
) -> Result<PhysicsContent, String> {
    let url = crate::remote::content_url(endpoint, &hint.key)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_CONTENT_BYTES as u64)
    {
        return Err("Content too large".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > MAX_CONTENT_BYTES {
            return Err("Content too large".into());
        }
        bytes.extend(chunk);
    }
    crate::remote::verify_content(hint, &bytes)
}
