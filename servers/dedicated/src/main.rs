use eldiron_server::{
    host::Host,
    transport::{self, TransportConfig},
};
use std::path::PathBuf;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Eldiron server: {error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), String> {
    let mut project = None;
    let mut state = None;
    let mut demo = false;
    let mut insecure = false;
    let mut bind = "127.0.0.1:8443".to_string();
    let mut cert = None;
    let mut key = None;
    let mut origins = vec![];
    let mut operations = eldiron_server::operations::Operations::default();
    let mut max_players = 64usize;
    let mut identity_config = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => project = Some(PathBuf::from(args.next().ok_or("Missing project")?)),
            "--state" => state = Some(PathBuf::from(args.next().ok_or("Missing state")?)),
            "--demo" => demo = true,
            "--insecure-loopback" => insecure = true,
            "--bind" => bind = args.next().ok_or("Missing bind address")?,
            "--tls-cert" => cert = Some(PathBuf::from(args.next().ok_or("Missing certificate")?)),
            "--tls-key" => key = Some(PathBuf::from(args.next().ok_or("Missing private key")?)),
            "--origin" => origins.push(args.next().ok_or("Missing browser origin")?),
            "--identity-config" => {
                identity_config = Some(PathBuf::from(
                    args.next()
                        .ok_or("Missing private identity configuration")?,
                ))
            }
            "--max-players" => {
                max_players = args
                    .next()
                    .ok_or("Missing player limit")?
                    .parse()
                    .map_err(|_| "Invalid player limit")?
            }
            "--metrics-file" => {
                operations.metrics_file =
                    Some(PathBuf::from(args.next().ok_or("Missing metrics file")?))
            }
            "--backup-dir" => {
                operations.backup_dir = Some(PathBuf::from(
                    args.next().ok_or("Missing backup directory")?,
                ))
            }
            "--backup-seconds" => {
                operations.backup_interval = std::time::Duration::from_secs(
                    args.next()
                        .ok_or("Missing backup interval")?
                        .parse()
                        .map_err(|_| "Invalid backup interval")?,
                )
            }
            "--backup-retain" => {
                operations.backup_retain = args
                    .next()
                    .ok_or("Missing retention count")?
                    .parse()
                    .map_err(|_| "Invalid retention count")?
            }
            "--help" => {
                println!(
                    "eldiron-server --project game.eldiron (--demo | --state world.save) [--bind 127.0.0.1:8443] (--tls-cert cert.pem --tls-key key.pem | --insecure-loopback) [--origin https://demo.example] [--max-players 1..64] [--identity-config private/identity.toml] [--metrics-file private/metrics.json] [--backup-dir private/backups --backup-seconds 300 --backup-retain 8]"
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown option {arg}")),
        }
    }
    if demo == state.is_some() {
        return Err("Choose exactly one: --demo or --state".into());
    }
    operations.validate()?;
    if !(1..=64).contains(&max_players) {
        return Err("Player limit must be 1..64".into());
    }
    if demo && operations.backup_dir.is_some() {
        return Err("Disposable demos do not create durable backups".into());
    }
    let project = project.ok_or("--project is required")?;
    if let Some(metrics) = &operations.metrics_file {
        let target = eldiron_server::operations::path_identity(metrics)?;
        if target == eldiron_server::operations::path_identity(&project)?
            || state.as_ref().is_some_and(|p| {
                eldiron_server::operations::path_identity(p).ok() == Some(target.clone())
            })
        {
            return Err("Metrics output must be separate from project and world save".into());
        }
    }
    let tls = match (cert, key, insecure) {
        (Some(cert), Some(key), false) => Some(transport::load_tls(&cert, &key)?),
        (None, None, true) => None,
        _ => return Err("Use TLS certificate/key or explicit --insecure-loopback".into()),
    };
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| e.to_string())?;
    if tls.is_none()
        && !listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .ip()
            .is_loopback()
    {
        return Err("Insecure test transport must bind loopback".into());
    }
    let bytes = std::fs::read(project).map_err(|e| e.to_string())?;
    let mut project = shared::project_io::decode_project(&bytes).map_err(|e| e.to_string())?;
    project.migrate_default_ruleset();
    let mut host = Host::open(project, state)
        .await
        .map_err(|e| e.to_string())?;
    host.set_player_limit(max_players)
        .map_err(|e| e.to_string())?;
    if let Some(path) = identity_config {
        host.configure_identity(eldiron_server::oidc::Broker::load(path).await?)?;
    }
    if operations.metrics_file.is_some() || operations.backup_dir.is_some() {
        host.configure_operations(operations)
            .map_err(|e| e.to_string())?;
        host.write_metrics_now();
    }
    println!(
        "Eldiron {:?} host listening on {}",
        host.world.mode(),
        listener.local_addr().unwrap()
    );
    transport::serve(
        listener,
        host,
        TransportConfig { tls, origins },
        shutdown_signal(),
    )
    .await
}

// Container/service managers normally send SIGTERM. It must use the same
// checkpoint/connection shutdown path as interactive Ctrl-C.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=terminate.recv()=>{} }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
