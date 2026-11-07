use eldiron_scepter::{RegionRef, ScepterCommand, ScepterLorebook};
use serde_json::json;
use std::io::Write;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
pub mod jobs;
mod transport;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

pub const SCEPTER_HOST: &str = "127.0.0.1";
pub const SCEPTER_PORT: u16 = 37687;

#[derive(Debug)]
pub enum ScepterEvent {
    PreparedEdit(crate::editor::scepter_staging::PreparedEdit),
    Execute {
        request: eldiron_scepter::ScepterRequest,
        job: Arc<jobs::Job>,
    },
    Ping {
        message: String,
        peer: String,
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
    tx: Option<Sender<ScepterEvent>>,
}

impl ScepterService {
    pub fn sender(&self) -> Option<Sender<ScepterEvent>> {
        self.tx.clone()
    }

    pub fn start() -> (Self, Receiver<ScepterEvent>) {
        let (tx, rx) = channel();
        let address = format!("{SCEPTER_HOST}:{SCEPTER_PORT}");
        let service = Self {
            address: address.clone(),
            started: true,
            tx: Some(tx.clone()),
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
                        tx: None,
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
    let registry = Arc::new(jobs::Registry::default());
    let active = Arc::new(AtomicUsize::new(0));
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if active.fetch_add(1, Ordering::AcqRel) >= 16 {
                    active.fetch_sub(1, Ordering::AcqRel);
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
                    let _ = write_json(
                        &mut stream,
                        503,
                        json!({"ok":false,"error":"Too many requests"}),
                    );
                    continue;
                }
                let tx = tx.clone();
                let registry = registry.clone();
                let active = active.clone();
                thread::spawn(move || {
                    handle_stream(stream, &tx, &registry);
                    active.fetch_sub(1, Ordering::AcqRel);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(e) => {
                let _ = tx.send(ScepterEvent::ServiceError(e.to_string()));
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
fn handle_stream(mut stream: TcpStream, tx: &Sender<ScepterEvent>, registry: &jobs::Registry) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let request = match transport::read(&mut stream) {
        Ok(v) => v,
        Err((code, error)) => {
            let _ = write_json(&mut stream, code, json!({"ok":false,"error":error}));
            return;
        }
    };
    let path = request.path.split('?').next().unwrap_or_default();
    if path == "/scepter/ping" {
        let _ = write_json(
            &mut stream,
            200,
            json!({"ok":true,"service":"eldiron_scepter","message":"pong"}),
        );
        return;
    }
    if request.method == "GET" && path == "/scepter/lorebook" {
        let _ = write_json(&mut stream, 200, json!(ScepterLorebook::built_in()));
        return;
    }
    if path == "/scepter/job" && matches!(request.method.as_str(), "GET" | "DELETE") {
        let id = transport::query(&request.path, "request_id").unwrap_or_default();
        match registry.get(&id) {
            Some(job) => {
                if request.method == "DELETE" {
                    if job.cancel() {
                        let _ = write_json(
                            &mut stream,
                            200,
                            json!({"ok":true,"status":"cancelled","request_id":id}),
                        );
                    } else {
                        let _ = write_json(
                            &mut stream,
                            409,
                            json!({"ok":false,"error":"Only queued requests can be cancelled","request_id":id}),
                        );
                    }
                    return;
                }
                let mut result = job.response(Duration::ZERO);
                result["request_id"] = json!(id);
                send_result(&mut stream, result);
            }
            None => {
                let _ = write_json(
                    &mut stream,
                    404,
                    json!({"ok":false,"error":"Unknown request_id"}),
                );
            }
        }
        return;
    }
    let mut command = if request.method == "POST" && path == "/scepter/command" {
        match serde_json::from_str::<eldiron_scepter::ScepterRequest>(&request.body) {
            Ok(v) => v,
            Err(e) => {
                let _ = write_json(&mut stream, 400, json!({"ok":false,"error":e.to_string()}));
                return;
            }
        }
    } else if request.method == "GET" {
        let region = transport::query(&request.path, "id")
            .map(RegionRef::id)
            .or_else(|| transport::query(&request.path, "name").map(RegionRef::name));
        let command = match path {
            "/scepter/project" => ScepterCommand::ProjectDescribe,
            "/scepter/tiles" => ScepterCommand::TileList(Default::default()),
            "/scepter/region" => ScepterCommand::RegionSnapshot(eldiron_scepter::RegionSnapshot {
                region,
                include_tiles: true,
            }),
            "/scepter/region/summary" => {
                ScepterCommand::RegionSummary(eldiron_scepter::RegionSummary {
                    region,
                    include_ascii: true,
                })
            }
            _ => {
                let _ = write_json(
                    &mut stream,
                    404,
                    json!({"ok":false,"error":"Unknown endpoint"}),
                );
                return;
            }
        };
        eldiron_scepter::ScepterRequest {
            request_id: None,
            session: None,
            revision: None,
            command,
        }
    } else {
        let _ = write_json(
            &mut stream,
            404,
            json!({"ok":false,"error":"Unknown endpoint"}),
        );
        return;
    };
    if !eldiron_scepter::creator_implements(command.command.name()) {
        let _ = write_json(
            &mut stream,
            501,
            json!({"ok":false,"error":"Command is planned, not implemented","command":command.command.name()}),
        );
        return;
    }
    if command.command.is_mutation()
        && (command.request_id.is_none() || command.session.is_none() || command.revision.is_none())
    {
        let _ = write_json(
            &mut stream,
            428,
            json!({"ok":false,"error":"Edits require request_id, session and revision from project.describe"}),
        );
        return;
    }
    let id = command
        .request_id
        .clone()
        .unwrap_or_else(|| theframework::prelude::Uuid::new_v4().to_string());
    if id.is_empty() || id.len() > 128 {
        let _ = write_json(
            &mut stream,
            400,
            json!({"ok":false,"error":"request_id must be 1..128 bytes"}),
        );
        return;
    }
    command.request_id = Some(id.clone());
    let payload = serde_json::to_string(&command).unwrap();
    match registry.submit(&id, payload, command.command.is_mutation()) {
        Ok((job, new)) => {
            if new
                && tx
                    .send(ScepterEvent::Execute {
                        request: command,
                        job: job.clone(),
                    })
                    .is_err()
            {
                job.finish(json!({"ok":false,"http_status":503,"error":"Creator is unavailable"}));
            }
            let mut result = job.response(Duration::from_secs(2));
            result["request_id"] = json!(id);
            send_result(&mut stream, result);
        }
        Err(error) => {
            let _ = write_json(
                &mut stream,
                if error.starts_with("request_id") {
                    409
                } else {
                    503
                },
                json!({"ok":false,"error":error}),
            );
        }
    }
}
fn send_result(stream: &mut TcpStream, mut result: serde_json::Value) {
    let status = result
        .get("http_status")
        .and_then(|v| v.as_u64())
        .unwrap_or(if result["ok"] == false { 422 } else { 200 }) as u16;
    result.as_object_mut().unwrap().remove("http_status");
    let _ = write_json(stream, status, result);
}
fn write_json(
    stream: &mut TcpStream,
    status: u16,
    value: serde_json::Value,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        408 => "Request Timeout",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        422 => "Unprocessable Content",
        428 => "Precondition Required",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let body = serde_json::to_string(&value)?;
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn exchange(
        tx: Sender<ScepterEvent>,
        registry: Arc<jobs::Registry>,
        method: &str,
        path: &str,
        body: serde_json::Value,
    ) -> (u16, serde_json::Value) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_stream(stream, &tx, &registry);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let body = body.to_string();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        server.join().unwrap();
        let (header, body) = response.split_once("\r\n\r\n").unwrap();
        (
            header.split_whitespace().nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(body).unwrap(),
        )
    }
    #[test]
    fn http_preconditions_errors_and_idempotent_retries() {
        let (tx, rx) = channel();
        let registry = Arc::new(jobs::Registry::default());
        let payload = json!({"command":"project.undo"});
        assert_eq!(
            exchange(
                tx.clone(),
                registry.clone(),
                "POST",
                "/scepter/command",
                payload
            )
            .0,
            428
        );
        assert!(rx.try_recv().is_err());
        let payload =
            json!({"request_id":"edit-1","session":"a","revision":"1","command":"project.undo"});
        let responder = thread::spawn(move || {
            let ScepterEvent::Execute { job, .. } = rx.recv().unwrap() else {
                panic!("Unexpected event")
            };
            assert!(job.begin());
            job.finish(json!({"ok":false,"error":"Nothing to undo"}));
            rx
        });
        let (status, result) = exchange(
            tx.clone(),
            registry.clone(),
            "POST",
            "/scepter/command",
            payload.clone(),
        );
        assert_eq!(status, 422);
        assert_eq!(result["ok"], false);
        let rx = responder.join().unwrap();
        assert_eq!(
            exchange(
                tx.clone(),
                registry.clone(),
                "POST",
                "/scepter/command",
                payload.clone()
            )
            .0,
            422
        );
        assert!(rx.try_recv().is_err());
        let mut changed = payload;
        changed["revision"] = json!("2");
        assert_eq!(
            exchange(tx, registry, "POST", "/scepter/command", changed).0,
            409
        );
    }
    #[test]
    fn http_timeout_is_pending_and_queued_cancel_prevents_execution() {
        let (tx, rx) = channel();
        let registry = Arc::new(jobs::Registry::default());
        let payload =
            json!({"request_id":"queued-1","session":"a","revision":"1","command":"project.undo"});
        let (status, result) = exchange(
            tx.clone(),
            registry.clone(),
            "POST",
            "/scepter/command",
            payload.clone(),
        );
        assert_eq!(status, 202);
        assert_eq!(result["status"], "queued");
        assert_eq!(
            exchange(
                tx.clone(),
                registry.clone(),
                "DELETE",
                "/scepter/job?request_id=queued-1",
                json!({})
            )
            .0,
            200
        );
        let ScepterEvent::Execute { job, .. } = rx.recv().unwrap() else {
            panic!("Unexpected event")
        };
        assert!(!job.begin());
        let (status, result) = exchange(tx, registry, "POST", "/scepter/command", payload);
        assert_eq!(status, 409);
        assert_eq!(result["status"], "cancelled");
        assert!(rx.try_recv().is_err());
    }
}
