use scenevm::run_scenevm_app;
use scenevm_unified_app::EldironPlayerApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let app = EldironPlayerApp::from_args(args);
    run_scenevm_app(app)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
