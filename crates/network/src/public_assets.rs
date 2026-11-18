//! Verified public release bootstrap and pack IO, independent of the authority.
use eldiron_runtime::Project;
use shared::public_client::{
    self,
    streaming::{self, Assembly, Pack, PackRef},
};
use std::sync::mpsc::{self, Receiver};
pub use streaming::PublicRegion;
#[derive(Clone, Debug)]
pub struct RegionAssets {
    pub source: String,
    pub regions: Vec<PackRef>,
}
impl RegionAssets {
    pub fn for_project(project: &Project) -> Result<Option<Self>, String> {
        let Some(release) = &project.client_release else {
            return Ok(None);
        };
        if release.manifest.version == 1 {
            return Ok(None);
        };
        Ok(Some(Self {
            source: release
                .stream_source
                .clone()
                .ok_or("Streamed release has no asset origin")?,
            regions: release
                .manifest
                .packs
                .iter()
                .filter(|p| p.map.is_some())
                .cloned()
                .collect(),
        }))
    }
}
pub fn pack_url(source: &str, p: &PackRef) -> Result<url::Url, String> {
    let base = url::Url::parse(source).map_err(|_| "Invalid public release origin")?;
    let loopback = matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    let scheme = base.scheme();
    if !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
        || !(scheme == "https"
            || scheme == "http" && loopback
            || scheme == "file" && cfg!(not(target_arch = "wasm32")))
    {
        return Err(
            "Public assets require HTTPS, a native release directory or explicit loopback HTTP"
                .into(),
        );
    }
    if p.sha256.len() != 64
        || !p
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid public asset digest".into());
    }
    base.join(&p.path()).map_err(|e| e.to_string())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn load_path(path: &std::path::Path) -> Result<Project, String> {
    use std::io::Read;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let project = public_client::load_player(&bytes, crate::PROTOCOL_VERSION)?;
    if project
        .client_release
        .as_ref()
        .is_none_or(|r| r.manifest.version == 1)
    {
        return Ok(project);
    }
    let source = url::Url::from_file_path(path.canonicalize().map_err(|e| e.to_string())?)
        .map_err(|_| "Invalid release path")?
        .to_string();
    let mut assembly = Assembly::new(&bytes, crate::PROTOCOL_VERSION)?;
    for pack in assembly.globals() {
        let path = pack_url(&source, &pack)?
            .to_file_path()
            .map_err(|_| "Invalid pack file path")?;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(pack.bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        assembly.accept(&pack, &bytes)?;
    }
    assembly.finish(source)
}
#[cfg(not(target_arch = "wasm32"))]
async fn fetch(source: &str, pack: &PackRef) -> Result<Vec<u8>, String> {
    use tokio::io::AsyncReadExt;
    let url = pack_url(source, pack)?;
    let work = async {
        let mut bytes = Vec::new();
        if url.scheme() == "file" {
            let file = tokio::fs::File::open(url.to_file_path().map_err(|_| "Invalid pack path")?)
                .await
                .map_err(|e| e.to_string())?;
            file.take(pack.bytes as u64 + 1)
                .read_to_end(&mut bytes)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| e.to_string())?;
            let mut response = client
                .get(url)
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            if response
                .content_length()
                .is_some_and(|n| n != pack.bytes as u64)
            {
                return Err("Public pack length mismatch".into());
            }
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                if bytes.len().saturating_add(chunk.len()) > pack.bytes {
                    return Err("Public pack exceeds declared length".into());
                }
                bytes.extend_from_slice(&chunk);
            }
        }
        Ok(bytes)
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), work)
        .await
        .map_err(|_| "Public asset fetch timed out".to_string())?
}
#[cfg(target_arch = "wasm32")]
async fn fetch(source: &str, pack: &PackRef) -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast;
    let url = pack_url(source, pack)?;
    let abort = crate::content_task::FetchAbort::new().map_err(js_error)?;
    let options = web_sys::RequestInit::new();
    options.set_signal(Some(&abort.signal()));
    options.set_redirect(web_sys::RequestRedirect::Error);
    options.set_credentials(web_sys::RequestCredentials::Omit);
    let response = wasm_bindgen_futures::JsFuture::from(
        web_sys::window()
            .ok_or("Browser window required")?
            .fetch_with_str_and_init(url.as_str(), &options),
    )
    .await
    .map_err(js_error)?
    .dyn_into::<web_sys::Response>()
    .map_err(js_error)?;
    if !response.ok() {
        return Err(format!(
            "Public asset unavailable: HTTP {}",
            response.status()
        ));
    }
    if response
        .headers()
        .get("content-length")
        .map_err(js_error)?
        .and_then(|s| s.parse::<usize>().ok())
        .is_some_and(|n| n != pack.bytes)
    {
        return Err("Public pack length mismatch".into());
    }
    // Read the stream with a hard byte ceiling, including chunked responses.
    let body = response.body().ok_or("Missing public asset body")?;
    let reader = body
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|e| js_error(e.into()))?;
    let result = async {
        let mut bytes = Vec::new();
        loop {
            let read = wasm_bindgen_futures::JsFuture::from(reader.read())
                .await
                .map_err(js_error)?;
            if js_sys::Reflect::get(&read, &"done".into())
                .map_err(js_error)?
                .as_bool()
                == Some(true)
            {
                break;
            }
            let chunk = js_sys::Uint8Array::new(
                &js_sys::Reflect::get(&read, &"value".into()).map_err(js_error)?,
            );
            if bytes.len().saturating_add(chunk.length() as usize) > pack.bytes {
                return Err("Public pack exceeds declared length".into());
            }
            let start = bytes.len();
            bytes.resize(start + chunk.length() as usize, 0);
            chunk.copy_to(&mut bytes[start..]);
        }
        Ok(bytes)
    }
    .await;
    if result.is_err() {
        let _ = reader.cancel();
    }
    reader.release_lock();
    result
}
#[cfg(target_arch = "wasm32")]
fn js_error(e: wasm_bindgen::JsValue) -> String {
    e.as_string()
        .unwrap_or_else(|| "Public asset download failed or timed out".into())
}
#[cfg(target_arch = "wasm32")]
pub async fn load_browser(
    bytes: &[u8],
    source: &str,
    mut progress: impl FnMut(usize, usize),
) -> Result<Project, String> {
    let project = public_client::load_player(bytes, crate::PROTOCOL_VERSION)?;
    if project
        .client_release
        .as_ref()
        .is_none_or(|r| r.manifest.version == 1)
    {
        return Ok(project);
    }
    let mut assembly = Assembly::new(bytes, crate::PROTOCOL_VERSION)?;
    let globals = assembly.globals();
    let mut completed = 0;
    for batch in globals.chunks(4) {
        let results =
            futures_util::future::try_join_all(batch.iter().map(|p| fetch(source, p))).await?;
        for (p, bytes) in batch.iter().zip(results) {
            assembly.accept(p, &bytes)?;
            completed += 1;
            progress(completed, globals.len());
        }
    }
    assembly.finish(source.into())
}
pub(crate) struct RegionTask {
    receiver: Receiver<Result<PublicRegion, String>>,
    #[cfg(not(target_arch = "wasm32"))]
    task: tokio::task::JoinHandle<()>,
    #[cfg(target_arch = "wasm32")]
    abort: futures_util::future::AbortHandle,
}
impl RegionTask {
    pub fn start(source: String, reference: PackRef) -> Result<Self, String> {
        let (send, receiver) = mpsc::sync_channel(1);
        let work = async move {
            let result = async {
                let bytes = fetch(&source, &reference).await?;
                match streaming::decode_pack(&reference, &bytes)? {
                    Pack::Region(r) => Ok(r),
                    _ => Err("Expected public region artwork".into()),
                }
            }
            .await;
            let _ = send.try_send(result);
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let task = tokio::runtime::Handle::try_current()
                .map_err(|_| "Public assets need an async runtime")?
                .spawn(work);
            Ok(Self { receiver, task })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (abort, registration) = futures_util::future::AbortHandle::new_pair();
            wasm_bindgen_futures::spawn_local(async {
                let _ = futures_util::future::Abortable::new(work, registration).await;
            });
            Ok(Self { receiver, abort })
        }
    }
    pub fn poll(&mut self) -> Option<Result<PublicRegion, String>> {
        match self.receiver.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(_) => Some(Err("Public region download stopped".into())),
        }
    }
}
impl Drop for RegionTask {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.task.abort();
        #[cfg(target_arch = "wasm32")]
        self.abort.abort();
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn pack_paths_are_origin_scoped_and_digest_only() {
        let p = PackRef {
            sha256: "a".repeat(64),
            bytes: 1,
            decoded_bytes: 1,
            map: None,
        };
        assert_eq!(
            pack_url("https://cdn.example/releases/one/game.eldiron-client", &p)
                .unwrap()
                .as_str(),
            format!("https://cdn.example/releases/one/{}", p.path())
        );
        for origin in [
            "http://cdn.example/game.eldiron-client",
            "https://user:pass@cdn.example/game.eldiron-client",
            "https://cdn.example/game.eldiron-client?token=secret",
            "https://cdn.example/game.eldiron-client#x",
        ] {
            assert!(pack_url(origin, &p).is_err());
        }
        let mut bad = p;
        bad.sha256 = "../private".into();
        assert!(pack_url("https://cdn.example/game.eldiron-client", &bad).is_err());
    }
    #[tokio::test]
    async fn dropping_a_region_job_cancels_its_http_socket() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut byte = [0u8; 1];
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                assert_eq!(socket.read(&mut byte).await.unwrap(), 1);
                headers.push(byte[0]);
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nx")
                .await
                .unwrap();
            let _ = started.send(());
            match tokio::time::timeout(std::time::Duration::from_secs(3), socket.read(&mut byte))
                .await
                .unwrap()
            {
                Ok(0) => true,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::BrokenPipe
                    ) =>
                {
                    true
                }
                _ => false,
            }
        });
        let job = RegionTask::start(
            format!("http://{addr}/game.eldiron-client"),
            PackRef {
                sha256: "a".repeat(64),
                bytes: 100,
                decoded_bytes: 100,
                map: Some(theframework_core::Uuid::nil()),
            },
        )
        .unwrap();
        ready.await.unwrap();
        drop(job);
        assert!(
            server.await.unwrap(),
            "cancelled owner must not leave a stalled HTTP read alive"
        );
    }
}
