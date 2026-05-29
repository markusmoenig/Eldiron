use eldiron_scepter::{
    AttributesGet, AttributesPatch, RegionPaintCells, RegionPaintRect, RegionRef,
    RegionRenderPreview, ScepterCommand, ScepterLorebook, ScriptGet, ScriptPatch, ScriptValidate,
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

pub const SCEPTER_HOST: &str = "127.0.0.1";
pub const SCEPTER_PORT: u16 = 37687;

#[derive(Debug)]
pub enum ScepterEvent {
    Ping {
        message: String,
        peer: String,
    },
    ProjectSnapshot {
        reply: Sender<serde_json::Value>,
    },
    ProjectUndo {
        reply: Sender<serde_json::Value>,
    },
    ProjectRedo {
        reply: Sender<serde_json::Value>,
    },
    TilesSnapshot {
        reply: Sender<serde_json::Value>,
    },
    RegionSnapshot {
        request: ScepterRegionRequest,
        reply: Sender<serde_json::Value>,
    },
    RegionSummary {
        request: ScepterRegionRequest,
        reply: Sender<serde_json::Value>,
    },
    RegionRenderPreview {
        command: RegionRenderPreview,
        reply: Sender<serde_json::Value>,
    },
    RegionPaintRect {
        command: RegionPaintRect,
        reply: Sender<serde_json::Value>,
    },
    RegionPaintCells {
        command: RegionPaintCells,
        reply: Sender<serde_json::Value>,
    },
    ScriptGet {
        command: ScriptGet,
        reply: Sender<serde_json::Value>,
    },
    ScriptPatch {
        command: ScriptPatch,
        reply: Sender<serde_json::Value>,
    },
    ScriptValidate {
        command: ScriptValidate,
        reply: Sender<serde_json::Value>,
    },
    AttributesGet {
        command: AttributesGet,
        reply: Sender<serde_json::Value>,
    },
    AttributesPatch {
        command: AttributesPatch,
        reply: Sender<serde_json::Value>,
    },
    ServiceError(String),
}

#[derive(Debug, Clone)]
pub struct ScepterRegionRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub include_tiles: bool,
    pub include_ascii: bool,
}

impl Default for ScepterRegionRequest {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            include_tiles: false,
            include_ascii: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScepterService {
    address: String,
    started: bool,
}

impl ScepterService {
    pub fn start() -> (Self, Receiver<ScepterEvent>) {
        let (tx, rx) = channel();
        let address = format!("{SCEPTER_HOST}:{SCEPTER_PORT}");
        let service = Self {
            address: address.clone(),
            started: true,
        };

        match TcpListener::bind(&address) {
            Ok(listener) => {
                if let Err(err) = listener.set_nonblocking(true) {
                    let _ = tx.send(ScepterEvent::ServiceError(format!(
                        "could not set {address} nonblocking: {err}"
                    )));
                }
                thread::spawn(move || run_listener(listener, tx));
            }
            Err(err) => {
                let _ = tx.send(ScepterEvent::ServiceError(format!(
                    "could not bind local Scepter API at {address}: {err}"
                )));
                return (
                    Self {
                        address,
                        started: false,
                    },
                    rx,
                );
            }
        }

        (service, rx)
    }

    pub fn status_line(&self) -> String {
        if self.started {
            format!(
                "Scepter API listening on http://{}/scepter/ping",
                self.address
            )
        } else {
            format!("Scepter API unavailable at {}", self.address)
        }
    }
}

fn run_listener(listener: TcpListener, tx: Sender<ScepterEvent>) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => handle_stream(stream, &tx),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = tx.send(ScepterEvent::ServiceError(format!("listener error: {err}")));
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
}

fn handle_stream(mut stream: TcpStream, tx: &Sender<ScepterEvent>) {
    let peer = stream
        .peer_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "local".to_string());
    let mut buffer = [0_u8; 16 * 1024];
    let size = match stream.read(&mut buffer) {
        Ok(size) => size,
        Err(err) => {
            let _ = write_json(
                &mut stream,
                400,
                json!({ "ok": false, "error": err.to_string() }),
            );
            return;
        }
    };
    let request = String::from_utf8_lossy(&buffer[..size]);
    let (method, path) = request
        .lines()
        .next()
        .and_then(parse_request_line)
        .unwrap_or(("GET", "/"));
    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default();

    match (method, path_without_query(path)) {
        ("GET", "/scepter/ping") | ("POST", "/scepter/ping") => {
            let message = ping_message(path, body);
            let _ = tx.send(ScepterEvent::Ping {
                message: message.clone(),
                peer: peer.clone(),
            });
            let _ = write_json(
                &mut stream,
                200,
                json!({
                    "ok": true,
                    "service": "eldiron_scepter",
                    "message": "pong",
                    "received": message,
                    "peer": peer,
                }),
            );
        }
        ("GET", "/scepter/lorebook") => {
            let _ = write_json(&mut stream, 200, json!(ScepterLorebook::built_in()));
        }
        ("POST", "/scepter/command") => {
            let Ok(command) = serde_json::from_str::<ScepterCommand>(body) else {
                let _ = write_json(
                    &mut stream,
                    400,
                    json!({ "ok": false, "error": "request body is not a valid ScepterCommand" }),
                );
                return;
            };

            handle_command(command, &mut stream, tx);
        }
        ("GET", "/scepter/project") => {
            let (reply_tx, reply_rx) = channel();
            if tx
                .send(ScepterEvent::ProjectSnapshot { reply: reply_tx })
                .is_err()
            {
                let _ = write_json(
                    &mut stream,
                    500,
                    json!({ "ok": false, "error": "Creator did not accept project snapshot request" }),
                );
                return;
            }

            match reply_rx.recv_timeout(Duration::from_secs(2)) {
                Ok(project) => {
                    let _ = write_json(
                        &mut stream,
                        200,
                        json!({
                            "ok": true,
                            "service": "eldiron_scepter",
                            "project": project,
                        }),
                    );
                }
                Err(err) => {
                    let _ = write_json(
                        &mut stream,
                        500,
                        json!({ "ok": false, "error": format!("project snapshot timed out: {err}") }),
                    );
                }
            }
        }
        ("GET", "/scepter/region") => {
            let request = region_request_from_query(path);
            request_creator_snapshot(
                &mut stream,
                tx,
                "region",
                |reply| ScepterEvent::RegionSnapshot { request, reply },
                "Creator did not accept region snapshot request",
                "region snapshot timed out",
            );
        }
        ("GET", "/scepter/region/summary") => {
            let request = region_request_from_query(path);
            request_creator_snapshot(
                &mut stream,
                tx,
                "summary",
                |reply| ScepterEvent::RegionSummary { request, reply },
                "Creator did not accept region summary request",
                "region summary timed out",
            );
        }
        ("GET", "/scepter/tiles") => {
            let (reply_tx, reply_rx) = channel();
            if tx
                .send(ScepterEvent::TilesSnapshot { reply: reply_tx })
                .is_err()
            {
                let _ = write_json(
                    &mut stream,
                    500,
                    json!({ "ok": false, "error": "Creator did not accept tile snapshot request" }),
                );
                return;
            }

            match reply_rx.recv_timeout(Duration::from_secs(2)) {
                Ok(tiles) => {
                    let _ = write_json(
                        &mut stream,
                        200,
                        json!({
                            "ok": true,
                            "service": "eldiron_scepter",
                            "tiles": tiles,
                        }),
                    );
                }
                Err(err) => {
                    let _ = write_json(
                        &mut stream,
                        500,
                        json!({ "ok": false, "error": format!("tile snapshot timed out: {err}") }),
                    );
                }
            }
        }
        _ => {
            let _ = write_json(
                &mut stream,
                404,
                json!({
                    "ok": false,
                    "error": "unknown Scepter endpoint",
                    "endpoints": [
                        "/scepter/ping",
                        "/scepter/lorebook",
                        "/scepter/command",
                        "/scepter/project",
                        "/scepter/region",
                        "/scepter/region/summary",
                        "/scepter/tiles"
                    ],
                }),
            );
        }
    }
}

fn handle_command(command: ScepterCommand, stream: &mut TcpStream, tx: &Sender<ScepterEvent>) {
    match command {
        ScepterCommand::ScepterHelp => {
            let lorebook = ScepterLorebook::built_in();
            let _ = write_json(
                stream,
                200,
                json!({
                    "ok": true,
                    "service": "eldiron_scepter",
                    "help": "Eldiron Scepter is Creator's local automation API. Use the Lorebook to discover commands, schemas, capabilities, and examples.",
                    "endpoints": [
                        "/scepter/ping",
                        "/scepter/lorebook",
                        "/scepter/command",
                        "/scepter/project",
                        "/scepter/region",
                        "/scepter/region/summary",
                        "/scepter/tiles"
                    ],
                    "commands": lorebook.list_commands(),
                }),
            );
        }
        ScepterCommand::ScepterListCommands => {
            let lorebook = ScepterLorebook::built_in();
            let _ = write_json(
                stream,
                200,
                json!({
                    "ok": true,
                    "service": "eldiron_scepter",
                    "commands": lorebook.list_commands(),
                }),
            );
        }
        ScepterCommand::ScepterDescribeCommand { name } => {
            let lorebook = ScepterLorebook::built_in();
            match lorebook.describe_command(&name) {
                Some(command) => {
                    let _ = write_json(
                        stream,
                        200,
                        json!({
                            "ok": true,
                            "service": "eldiron_scepter",
                            "command": command,
                        }),
                    );
                }
                None => {
                    let _ = write_json(
                        stream,
                        404,
                        json!({
                            "ok": false,
                            "error": format!("unknown Scepter command: {name}"),
                        }),
                    );
                }
            }
        }
        ScepterCommand::ProjectDescribe => request_creator_snapshot(
            stream,
            tx,
            "project",
            |reply| ScepterEvent::ProjectSnapshot { reply },
            "Creator did not accept project snapshot request",
            "project snapshot timed out",
        ),
        ScepterCommand::ProjectUndo => request_creator_snapshot(
            stream,
            tx,
            "result",
            |reply| ScepterEvent::ProjectUndo { reply },
            "Creator did not accept undo request",
            "undo timed out",
        ),
        ScepterCommand::ProjectRedo => request_creator_snapshot(
            stream,
            tx,
            "result",
            |reply| ScepterEvent::ProjectRedo { reply },
            "Creator did not accept redo request",
            "redo timed out",
        ),
        ScepterCommand::RegionSnapshot(params) => {
            let mut request = ScepterRegionRequest {
                include_tiles: params.include_tiles,
                ..ScepterRegionRequest::default()
            };
            if let Some(region) = params.region {
                match region {
                    RegionRef::Id { id } => request.id = Some(id),
                    RegionRef::Name { name } => request.name = Some(name),
                }
            }
            request_creator_snapshot(
                stream,
                tx,
                "region",
                |reply| ScepterEvent::RegionSnapshot { request, reply },
                "Creator did not accept region snapshot request",
                "region snapshot timed out",
            )
        }
        ScepterCommand::RegionSummary(params) => {
            let mut request = ScepterRegionRequest {
                include_ascii: params.include_ascii,
                ..ScepterRegionRequest::default()
            };
            if let Some(region) = params.region {
                match region {
                    RegionRef::Id { id } => request.id = Some(id),
                    RegionRef::Name { name } => request.name = Some(name),
                }
            }
            request_creator_snapshot(
                stream,
                tx,
                "summary",
                |reply| ScepterEvent::RegionSummary { request, reply },
                "Creator did not accept region summary request",
                "region summary timed out",
            )
        }
        ScepterCommand::RegionRenderPreview(command) => request_creator_snapshot(
            stream,
            tx,
            "preview",
            |reply| ScepterEvent::RegionRenderPreview { command, reply },
            "Creator did not accept region preview request",
            "region preview timed out",
        ),
        ScepterCommand::RegionPaintRect(command) => request_creator_snapshot(
            stream,
            tx,
            "result",
            |reply| ScepterEvent::RegionPaintRect { command, reply },
            "Creator did not accept region paint request",
            "region paint timed out",
        ),
        ScepterCommand::RegionPaintCells(command) => request_creator_snapshot(
            stream,
            tx,
            "result",
            |reply| ScepterEvent::RegionPaintCells { command, reply },
            "Creator did not accept region paint request",
            "region paint timed out",
        ),
        ScepterCommand::TileList(_) => request_creator_snapshot(
            stream,
            tx,
            "tiles",
            |reply| ScepterEvent::TilesSnapshot { reply },
            "Creator did not accept tile snapshot request",
            "tile snapshot timed out",
        ),
        ScepterCommand::ScriptGet(command) => request_creator_snapshot(
            stream,
            tx,
            "script",
            |reply| ScepterEvent::ScriptGet { command, reply },
            "Creator did not accept script read request",
            "script read timed out",
        ),
        ScepterCommand::ScriptPatch(command) => request_creator_snapshot(
            stream,
            tx,
            "result",
            |reply| ScepterEvent::ScriptPatch { command, reply },
            "Creator did not accept script patch request",
            "script patch timed out",
        ),
        ScepterCommand::ScriptValidate(command) => request_creator_snapshot(
            stream,
            tx,
            "validation",
            |reply| ScepterEvent::ScriptValidate { command, reply },
            "Creator did not accept script validation request",
            "script validation timed out",
        ),
        ScepterCommand::AttributesGet(command) => request_creator_snapshot(
            stream,
            tx,
            "attributes",
            |reply| ScepterEvent::AttributesGet { command, reply },
            "Creator did not accept attributes read request",
            "attributes read timed out",
        ),
        ScepterCommand::AttributesPatch(command) => request_creator_snapshot(
            stream,
            tx,
            "result",
            |reply| ScepterEvent::AttributesPatch { command, reply },
            "Creator did not accept attributes patch request",
            "attributes patch timed out",
        ),
        command => {
            let _ = write_json(
                stream,
                501,
                json!({
                    "ok": false,
                    "service": "eldiron_scepter",
                    "command": command.name(),
                    "error": "command is in the Lorebook but is not executable yet",
                }),
            );
        }
    }
}

fn request_creator_snapshot(
    stream: &mut TcpStream,
    tx: &Sender<ScepterEvent>,
    response_key: &str,
    event: impl FnOnce(Sender<serde_json::Value>) -> ScepterEvent,
    send_error: &str,
    timeout_error: &str,
) {
    let (reply_tx, reply_rx) = channel();
    if tx.send(event(reply_tx)).is_err() {
        let _ = write_json(stream, 500, json!({ "ok": false, "error": send_error }));
        return;
    }

    match reply_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(value) => {
            let mut body = json!({
                "ok": true,
                "service": "eldiron_scepter",
            });
            if let Some(object) = body.as_object_mut() {
                object.insert(response_key.to_string(), value);
            }
            let _ = write_json(stream, 200, body);
        }
        Err(err) => {
            let _ = write_json(
                stream,
                500,
                json!({ "ok": false, "error": format!("{timeout_error}: {err}") }),
            );
        }
    }
}

fn parse_request_line(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    Some((method, path))
}

fn path_without_query(path: &str) -> &str {
    path.split_once('?').map(|(path, _)| path).unwrap_or(path)
}

fn ping_message(path: &str, body: &str) -> String {
    if let Some(query) = path.split_once('?').map(|(_, query)| query) {
        for part in query.split('&') {
            if let Some(value) = part.strip_prefix("message=") {
                return value.replace('+', " ");
            }
        }
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(message) = value.get("message").and_then(serde_json::Value::as_str)
    {
        return message.to_string();
    }

    body.trim().to_string()
}

fn region_request_from_query(path: &str) -> ScepterRegionRequest {
    let mut request = ScepterRegionRequest::default();
    let Some(query) = path.split_once('?').map(|(_, query)| query) else {
        return request;
    };

    for part in query.split('&') {
        if let Some(value) = part.strip_prefix("id=") {
            request.id = Some(value.replace('+', " "));
        } else if let Some(value) = part.strip_prefix("name=") {
            request.name = Some(value.replace('+', " "));
        } else if let Some(value) = part.strip_prefix("include_tiles=") {
            request.include_tiles = matches!(value, "1" | "true" | "yes");
        } else if let Some(value) = part.strip_prefix("include_ascii=") {
            request.include_ascii = matches!(value, "1" | "true" | "yes");
        }
    }

    request
}

fn write_json(
    stream: &mut TcpStream,
    status: u16,
    value: serde_json::Value,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        501 => "Not Implemented",
        _ => "Internal Server Error",
    };
    let body = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string());
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())
}
