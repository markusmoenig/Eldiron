#[cfg(target_arch = "wasm32")]
fn main() {}
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(error) = run() {
        eprintln!("Client export: {error}");
        std::process::exit(1);
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn run() -> Result<(), String> {
    use std::{io::Write, path::PathBuf};
    let (mut project, mut output, mut endpoint, mut deployment) = (None, None, None, None);
    let mut streamed = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--streamed-dir" => {
                streamed = Some(PathBuf::from(
                    args.next().ok_or("Missing streamed release directory")?,
                ))
            }
            "--project" => project = Some(PathBuf::from(args.next().ok_or("Missing project")?)),
            "--output" => output = Some(PathBuf::from(args.next().ok_or("Missing output")?)),
            "--endpoint" => endpoint = Some(args.next().ok_or("Missing endpoint")?),
            "--deployment-id" => {
                deployment = Some(args.next().ok_or("Missing deployment identity")?)
            }
            "--help" => {
                println!(
                    "eldiron-client-export --project game.eldiron (--output game.eldiron-client | --streamed-dir release-directory) [--endpoint wss://host/play] [--deployment-id public-id]\nMultiplayer display content only. Output must be a fresh .eldiron-client path. Authored source and saves are not modified."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown option {arg}")),
        }
    }
    let project = project.ok_or("--project is required")?;
    if let Some(directory) = streamed {
        if output.is_some() {
            return Err("Choose --output or --streamed-dir".into());
        }
        if directory.exists() {
            return Err("Streamed release directory exists; choose a fresh path".into());
        }
        let source = std::fs::read(&project).map_err(|e| e.to_string())?;
        let bundle = eldiron_network::public_client::export_streamed_source(
            &source,
            endpoint.as_deref(),
            deployment.as_deref(),
        )?;
        let parent = directory
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        std::fs::create_dir(directory.join("packs")).map_err(|e| e.to_string())?;
        let write = |path: &std::path::Path, bytes: &[u8]| -> Result<(), String> {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            f.write_all(bytes)
                .and_then(|_| f.sync_all())
                .map_err(|e| e.to_string())
        };
        for (path, bytes) in &bundle.packs {
            write(&directory.join(path), bytes)?;
        }
        #[cfg(unix)]
        std::fs::File::open(directory.join("packs"))
            .and_then(|d| d.sync_all())
            .map_err(|e| e.to_string())?;
        let bootstrap = directory.join("game.eldiron-client");
        // Publish the bootstrap only when every referenced immutable pack exists.
        let mut temporary =
            tempfile::NamedTempFile::new_in(&directory).map_err(|e| e.to_string())?;
        temporary
            .write_all(&bundle.bootstrap)
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        temporary
            .persist_noclobber(&bootstrap)
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            std::fs::File::open(&directory)
                .and_then(|d| d.sync_all())
                .map_err(|e| e.to_string())?;
            std::fs::File::open(parent)
                .and_then(|d| d.sync_all())
                .map_err(|e| e.to_string())?;
        }
        println!(
            "Streamed public client: {} ({} bootstrap bytes, {} immutable packs)",
            bootstrap.display(),
            bundle.bootstrap.len(),
            bundle.packs.len()
        );
        return Ok(());
    }
    let output = output.ok_or("--output is required")?;
    if output.extension().is_none_or(|e| e != "eldiron-client") {
        return Err("Output must use .eldiron-client".into());
    }
    if output.exists() {
        return Err("Output exists; choose a fresh release path".into());
    }
    let source = std::fs::read(&project).map_err(|e| e.to_string())?;
    let bytes = eldiron_network::public_client::export_source(
        &source,
        endpoint.as_deref(),
        deployment.as_deref(),
    )?;
    let loaded = eldiron_network::public_client::load(&bytes)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist_noclobber(&output).map_err(|e| e.to_string())?;
    println!(
        "Public client: {} ({} bytes), server content {}",
        output.display(),
        bytes.len(),
        loaded
            .client_release
            .as_ref()
            .unwrap()
            .manifest
            .content_revision
    );
    Ok(())
}
