#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    scenevm_unified_app::native_player::desktop::run(std::env::args_os())
}
// Browser host convergence is tracked separately; the old wasm binary had no
// bootstrap. The shared presentation/runtime library still compiles for WebGPU.
#[cfg(target_arch = "wasm32")]
fn main() {}
