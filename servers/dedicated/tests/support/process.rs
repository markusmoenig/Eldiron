use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};
use theframework_core::Uuid;
use tokio_rustls::rustls;
pub struct Fixture {
    pub dir: PathBuf,
    pub project: PathBuf,
    pub tls: Arc<rustls::ClientConfig>,
}
impl Fixture {
    pub fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("eldiron-release-{}", Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let project =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_projects/3DStarter.eldiron");
        let status = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-sha256",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=localhost",
                "-addext",
                "subjectAltName=DNS:localhost",
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-keyout",
            ])
            .arg(dir.join("key.pem"))
            .arg("-out")
            .arg(dir.join("cert.pem"))
            .output()
            .unwrap();
        assert!(status.status.success());
        let pem = fs::read(dir.join("cert.pem")).unwrap();
        let der = rustls_pemfile::certs(&mut &pem[..])
            .next()
            .unwrap()
            .unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(der).unwrap();
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        Self {
            dir,
            project,
            tls: Arc::new(tls),
        }
    }
    pub async fn start(&self, state: &Path, label: &str) -> Server {
        self.start_configured(state, label, false).await
    }
    pub async fn start_configured(&self, state: &Path, label: &str, backups: bool) -> Server {
        let log = self.dir.join(format!("{label}.log"));
        let file = fs::File::create(&log).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_eldiron-server"));
        if backups {
            command
                .arg("--backup-dir")
                .arg(self.dir.join("backups"))
                .args(["--backup-seconds", "30", "--backup-retain", "3"]);
        }
        let child = command
            .arg("--project")
            .arg(&self.project)
            .arg("--state")
            .arg(state)
            .args(["--bind", "127.0.0.1:0", "--max-players", "3", "--tls-cert"])
            .arg(self.dir.join("cert.pem"))
            .arg("--tls-key")
            .arg(self.dir.join("key.pem"))
            .arg("--metrics-file")
            .arg(self.dir.join(format!("{label}.metrics.json")))
            .stdout(Stdio::from(file.try_clone().unwrap()))
            .stderr(Stdio::from(file))
            .spawn()
            .unwrap();
        let mut server = Server { child, port: 0 };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
        loop {
            let text = fs::read_to_string(&log).unwrap();
            if let Some(port) = text
                .lines()
                .find_map(|l| l.split("host listening on 127.0.0.1:").nth(1))
                .and_then(|s| s.parse::<u16>().ok())
            {
                server.port = port;
                return server;
            }
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "server exited: {text}"
            );
            assert!(
                tokio::time::Instant::now() < deadline,
                "server startup timed out: {text}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}
pub struct Server {
    child: Child,
    pub port: u16,
}
impl Server {
    pub fn id(&self) -> u32 {
        self.child.id()
    }
    pub fn assert_alive(&mut self) {
        assert!(
            self.child.try_wait().unwrap().is_none(),
            "server unexpectedly exited"
        );
    }
    pub fn kill(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
    #[cfg(unix)]
    pub async fn terminate(&mut self) {
        let result = Command::new("kill")
            .arg("-TERM")
            .arg(self.child.id().to_string())
            .status()
            .unwrap();
        assert!(result.success());
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "SIGTERM did not exit cleanly: {status}");
                return;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "shutdown did not finish"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
