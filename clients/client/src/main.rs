#![windows_subsystem = "windows"]
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    scenevm_unified_app::native_player::desktop::run(std::env::args_os())
}
#[cfg(target_arch = "wasm32")]
include!("browser_reference.rs");
