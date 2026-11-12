use scenevm::run_scenevm_app;
use scenevm_unified_app::EldironPlayerApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let render_debug = args.iter().any(|arg| arg == "--render-debug")
        || std::env::var("ELDIRON_RENDER_DEBUG")
            .map(|v| v != "0")
            .unwrap_or(false);
    if render_debug {
        write_startup_render_debug_log(&args);
    }
    let app = EldironPlayerApp::from_args(args);
    run_scenevm_app(app)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_startup_render_debug_log(args: &[String]) {
    use std::io::Write;

    let message = format!(
        "[RenderDebug][client-wgpu-main] startup version={} args={:?} cwd={:?} exe={:?} temp={:?}",
        env!("CARGO_PKG_VERSION"),
        args,
        std::env::current_dir().ok(),
        std::env::current_exe().ok(),
        std::env::temp_dir()
    );
    eprintln!("{message}");

    let mut paths = Vec::new();
    paths.push(std::path::PathBuf::from("eldiron-render-debug.log"));
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        paths.push(parent.join("eldiron-render-debug.log"));
    }
    paths.push(std::env::temp_dir().join("eldiron-render-debug.log"));

    let mut seen = std::collections::HashSet::new();
    for path in paths {
        let key = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(key) {
            continue;
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{message}");
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
